using System;
using System.Drawing;

using System.IO;
using System.Management;
using System.Runtime.InteropServices;
using System.Windows.Forms;

class MouseControls : ApplicationContext {
    delegate IntPtr Hook(int code, IntPtr message, IntPtr data);
    [DllImport("user32", SetLastError=true)] static extern IntPtr SetWindowsHookEx(int kind, Hook callback, IntPtr module, uint thread);
    [DllImport("kernel32")] static extern IntPtr GetModuleHandle(string name);
    [DllImport("user32")] static extern bool UnhookWindowsHookEx(IntPtr hook);
    [DllImport("user32")] static extern IntPtr CallNextHookEx(IntPtr hook,int code,IntPtr message,IntPtr data);
    [DllImport("user32")] static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
    [DllImport("user32")] static extern bool SetProcessDPIAware();

    readonly Hook callback;
    readonly Control dispatcher = new Control();
    readonly Icon appIcon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
    readonly NotifyIcon tray = new NotifyIcon();
    readonly string settings = Path.Combine(Application.LocalUserAppDataPath,"buttons.txt");
    readonly string[] modes = {"Volume","Brightness"};
    readonly SettingsWindow settingsWindow;
    bool snippingEnabled=true;
    IntPtr hook;
    int held, started, remainder;
    bool chord, scrolled, snipOnRelease;
    [DllImport("user32")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32")] static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);

    static void Navigate(int button){
        IntPtr window=GetForegroundWindow();
        if(window==IntPtr.Zero) return;
        // WM_APPCOMMAND, mouse origin, browser Back (1) or Forward (2).
        PostMessage(window,0x319,window,(IntPtr)((0x8000 | button)<<16));
    }
    MouseControls() {
        var handle = dispatcher.Handle;
        if(File.Exists(settings)){
            var saved=File.ReadAllLines(settings);
            snippingEnabled=saved.Length<3 || saved[2]!="Off";
            for(int i=0;i<Math.Min(2,saved.Length);i++)
                if(saved[i]=="Volume" || saved[i]=="Brightness") modes[i]=saved[i];
        }
        settingsWindow=new SettingsWindow(modes,snippingEnabled,SaveChoice);
        settingsWindow.Icon=appIcon;
        var menu=new ContextMenuStrip();
        menu.Items.Add("Settings",null,delegate { settingsWindow.Open(); });
        menu.Items.Add("Exit",null,delegate { ExitThread(); });
        callback=OnMouse;
        hook=SetWindowsHookEx(14,callback,GetModuleHandle(null),0);
        if(hook==IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        tray.Icon=appIcon; tray.Text="Mouse Controls"; tray.ContextMenuStrip=menu; tray.Visible=true;
        tray.MouseClick+=delegate(object sender, MouseEventArgs e){if(e.Button==MouseButtons.Left) settingsWindow.Open();};
        settingsWindow.Open();
    }
    void SaveChoice(int index,string value){
        if(index==2) snippingEnabled=value!="Off";
        else modes[index]=value;
        try{File.WriteAllLines(settings,new[]{modes[0],modes[1],snippingEnabled?"Snipping Tool":"Off"});}
        catch(IOException ex){Notify("Could not save settings. "+ex.Message);}
        catch(UnauthorizedAccessException ex){Notify("Could not save settings. "+ex.Message);}
    }
    void Notify(string message){tray.ShowBalloonTip(2500,"Mouse Controls",message,ToolTipIcon.Info);}
    IntPtr OnMouse(int code, IntPtr message, IntPtr data){
        int msg=message.ToInt32();
        if(code>=0 && (msg==0x20B || msg==0x20C || msg==0x20A) && (Marshal.ReadInt32(data,12)&1)==0){
            int value=Marshal.ReadInt32(data,8)>>16;
            int now=Marshal.ReadInt32(data,16);
            uint elapsed=unchecked((uint)(now-started));
            if(msg==0x20A){
                if(!chord && held!=0){
                    scrolled=true;
                    remainder+=value;
                    int steps=remainder/120; remainder%=120;
                    string mode=modes[held==1?0:1];
                    if(steps!=0) dispatcher.BeginInvoke(new Action(delegate {Adjust(mode,steps);}));
                    return (IntPtr)1;
                }
            } else if(value==1 || value==2){
                if(msg==0x20B){
                    if(held==0){started=now; remainder=0; scrolled=false;}
                    if((held&value)==0 && held!=0 && !chord){
                        chord=true;
                        snipOnRelease=snippingEnabled && elapsed<=300;
                    }
                    held|=value;
                    // Consume both edges so a gesture never leaves Windows holding a button.
                } else if((held&value)!=0){
                    bool tap=!chord && !scrolled;
                    held&=~value;
                    if(tap) Navigate(value);
                    if(held==0){
                        chord=false;
                        if(snipOnRelease){
                            snipOnRelease=false;
                            dispatcher.BeginInvoke(new Action(OpenSnippingOverlay));
                        }
                    }
                } else return CallNextHookEx(hook,code,message,data);
                return (IntPtr)1;
            }
        }
        return CallNextHookEx(hook,code,message,data);
    }
    void Adjust(string mode,int steps){
        if(mode=="Volume"){
            byte key=(byte)(steps>0?175:174);
            for(int i=0;i<Math.Abs(steps);i++){keybd_event(key,0,0,UIntPtr.Zero);keybd_event(key,0,2,UIntPtr.Zero);}
            return;
        }
        try{
            using(var reader=new ManagementObjectSearcher(@"root\WMI","SELECT * FROM WmiMonitorBrightness WHERE Active=TRUE"))
            using(var writer=new ManagementObjectSearcher(@"root\WMI","SELECT * FROM WmiMonitorBrightnessMethods WHERE Active=TRUE"))
            using(var panels=reader.Get())
            using(var methods=writer.Get()){
                bool changed=false;
                foreach(ManagementObject panel in panels)
                    foreach(ManagementObject method in methods)
                        if(Equals(panel["InstanceName"],method["InstanceName"])){
                            byte level=(byte)Math.Max(0,Math.Min(100,(byte)panel["CurrentBrightness"]+5*steps));
                            method.InvokeMethod("WmiSetBrightness",new object[]{(uint)0,level});

                            changed=true;
                        }
                if(changed) NativeBrightness.Show(); else Notify("Windows found no compatible brightness control for this display.");
            }
        }catch(Exception ex){Notify("Brightness unavailable. "+ex.Message);}
    }
    void OpenSnippingOverlay(){
        try{
            System.Diagnostics.Process.Start("ms-screenclip:");
        }catch(Exception ex){Notify("Could not open Windows screen snipping. "+ex.Message);}
    }
    protected override void ExitThreadCore(){UnhookWindowsHookEx(hook);settingsWindow.Dispose();tray.Dispose();appIcon.Dispose();dispatcher.Dispose();base.ExitThreadCore();}
    [STAThread] static void Main(){
        bool first;
        using(var mutex=new System.Threading.Mutex(true,"Local\\MouseControls.SideButtons",out first)){
            if(!first){SettingsWindow.PostMessage((IntPtr)0xffff,SettingsWindow.OpenMessage,IntPtr.Zero,IntPtr.Zero);return;}
            SetProcessDPIAware();Application.EnableVisualStyles();
            try{Application.Run(new MouseControls());}catch(Exception ex){MessageBox.Show(ex.Message,"Mouse Controls");}
        }
    }
}



static class NativeBrightness {
    [ComImport, Guid("6D5140C1-7436-11CE-8034-00AA006009FA"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IServiceProvider {
        void QueryService(ref Guid service, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out IFlyoutDisplay display);
    }
    [ComImport, Guid("41F9D2FB-7834-4AB6-8B1B-73E74064B465"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IFlyoutDisplay { void ShowFlyout(int kind, uint flags); }

    [DllImport("user32", CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string className, string title);
    [DllImport("user32", CharSet=CharSet.Unicode)] static extern uint RegisterWindowMessage(string name);
    [DllImport("user32")] static extern bool PostMessage(IntPtr window, uint message, IntPtr command, IntPtr data);
    static readonly uint shellHook = RegisterWindowMessage("SHELLHOOK");

    public static void Show(){
        object shell = null;
        IFlyoutDisplay display = null;
        try {
            shell = Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("C2F03A33-21F5-47FA-B4BB-156362A2F239")));
            Guid iid = typeof(IFlyoutDisplay).GUID;
            Guid service = iid;
            ((IServiceProvider)shell).QueryService(ref service, ref iid, out display);
            display.ShowFlyout(3, 0); // Windows 11 display brightness
            IntPtr host = FindWindow("NativeHWNDHost", "");
            if(host != IntPtr.Zero) PostMessage(host, shellHook, (IntPtr)0x38, IntPtr.Zero);
        } finally {
            if(display != null) Marshal.ReleaseComObject(display);
            if(shell != null) Marshal.ReleaseComObject(shell);
        }
    }
}



