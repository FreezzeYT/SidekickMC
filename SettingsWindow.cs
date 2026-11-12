using System;
using System.Drawing;
using System.Windows.Forms;

sealed class SettingsWindow : Form {
    readonly Color canvas = Color.FromArgb(21, 24, 29);
    readonly Color card = Color.FromArgb(32, 37, 44);
    readonly Color muted = Color.FromArgb(182, 192, 205);
    readonly Color accent = Color.FromArgb(156, 232, 206);
    readonly CheckBox startupToggle;
    bool updatingStartup;
    internal static readonly uint OpenMessage = RegisterWindowMessage("MouseControls.OpenSettings.1");
    [System.Runtime.InteropServices.DllImport("user32", CharSet=System.Runtime.InteropServices.CharSet.Unicode)]
    static extern uint RegisterWindowMessage(string name);
    [System.Runtime.InteropServices.DllImport("user32")]
    internal static extern bool PostMessage(IntPtr hwnd,uint message,IntPtr w,IntPtr l);

    internal SettingsWindow(string[] modes, bool snip, bool startWithWindows, Action<int,string> save, Action<bool> saveStartup){
        SuspendLayout();
        Text = "Mouse Controls";
        Font = new Font("Segoe UI", 10);
        BackColor = canvas;
        ForeColor = Color.White;
        AutoScaleDimensions = new SizeF(96,96);
        AutoScaleMode = AutoScaleMode.Dpi;
        ClientSize = new Size(596, 480);
        FormBorderStyle = FormBorderStyle.FixedSingle;
        MaximizeBox = false;
        StartPosition = FormStartPosition.CenterScreen;
        AddLabel(this,"Mouse controls",24,18,520,48,24,Color.White,true);
        AddLabel(this,"Hold a side button and scroll to adjust its assigned control.",26,72,553,28,10,muted,false);
        AddRow(110,"Back button","Hold and scroll",new[]{"Volume","Brightness"},modes[0],0,save);
        AddRow(186,"Forward button","Hold and scroll",new[]{"Volume","Brightness"},modes[1],1,save);
        AddRow(262,"Both buttons","Press together",new[]{"Snipping Tool","Off"},snip?"Snipping Tool":"Off",2,save);
        var startupRow=new Panel { BackColor=card, Bounds=new Rectangle(24,338,548,58), AccessibleName="Startup" };
        Controls.Add(startupRow);
        AddLabel(startupRow,"Start with Windows",16,8,260,26,12,Color.White,true);
        AddLabel(startupRow,"Run after you sign in",16,31,260,20,9,muted,false);
        startupToggle=new CheckBox { Text="On", Appearance=Appearance.Button, TextAlign=ContentAlignment.MiddleCenter,
            FlatStyle=FlatStyle.Flat, AutoSize=false, Bounds=new Rectangle(422,10,110,38), BackColor=canvas };
        startupToggle.FlatAppearance.BorderSize=0;
        startupToggle.FlatAppearance.CheckedBackColor=accent;
        startupToggle.Checked=startWithWindows;
        StyleToggle(startupToggle);
        startupToggle.CheckedChanged+=delegate { StyleToggle(startupToggle); if(!updatingStartup) saveStartup(startupToggle.Checked); };
        startupRow.Controls.Add(startupToggle);
        AddLabel(this,"Changes save automatically. Close this window to keep using your mouse.",26,420,546,44,10,muted,false);
        ResumeLayout();
    }

    void StyleToggle(CheckBox toggle){
        toggle.Text=toggle.Checked?"On":"Off";
        toggle.ForeColor=toggle.Checked?canvas:Color.White;
        toggle.FlatAppearance.MouseOverBackColor=toggle.Checked?accent:Color.FromArgb(73,85,96);
    }
    internal void SetStartWithWindows(bool enabled){
        updatingStartup=true;
        startupToggle.Checked=enabled;
        updatingStartup=false;
    }

    void AddLabel(Control parent,string text,int x,int y,int width,int height,float size,Color color,bool bold){
        var label=new Label { Text=text, ForeColor=color, BackColor=Color.Transparent,
            Font=new Font(Font.FontFamily,size,bold?FontStyle.Bold:FontStyle.Regular),
            Bounds=new Rectangle(x,y,width,height), TextAlign=ContentAlignment.MiddleLeft };
        parent.Controls.Add(label);
    }

    void AddRow(int y,string title,string hint,string[] options,string selected,int index,Action<int,string> save){
        var row=new Panel { BackColor=card, Bounds=new Rectangle(24,y,548,70), AccessibleName=title };
        Controls.Add(row);
        AddLabel(row,title,16,9,210,27,12,Color.White,true);
        AddLabel(row,hint,16,37,210,21,9,muted,false);

        for(int i=0;i<options.Length;i++){
            var choice=new RadioButton { Text=options[i], Appearance=Appearance.Button,
                TextAlign=ContentAlignment.MiddleCenter, FlatStyle=FlatStyle.Flat, AutoSize=false,
                Bounds=new Rectangle(258+i*135,16,130,38), AccessibleName=title+" - "+options[i] };

            choice.FlatAppearance.BorderSize=0;
            choice.FlatAppearance.CheckedBackColor=accent;
            choice.FlatAppearance.MouseOverBackColor=Color.FromArgb(73,85,96);
            choice.BackColor=canvas;
            row.Controls.Add(choice);
            choice.Checked=choice.Text==selected;
            choice.ForeColor=choice.Checked?canvas:Color.White;
            choice.CheckedChanged+=delegate {
                choice.ForeColor=choice.Checked?canvas:Color.White;
                choice.FlatAppearance.MouseOverBackColor=choice.Checked?accent:Color.FromArgb(73,85,96);
                if(choice.Checked) save(index,choice.Text);
            };
            choice.FlatAppearance.MouseOverBackColor=choice.Checked?accent:Color.FromArgb(73,85,96);
        }
    }
    internal void Open(){
        Show();
        if(WindowState==FormWindowState.Minimized) WindowState=FormWindowState.Normal;
        Activate();
    }
    protected override void WndProc(ref Message message){
        if(message.Msg==OpenMessage) Open();
        base.WndProc(ref message);
    }
    protected override void OnFormClosing(FormClosingEventArgs e){
        if(e.CloseReason==CloseReason.UserClosing){e.Cancel=true;Hide();}
        base.OnFormClosing(e);
    }
}

