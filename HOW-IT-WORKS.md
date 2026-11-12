# How SidekickMC works

## Language and structure

The application is written in C# using .NET Framework and Windows Forms. [SidekickMC.cs](SidekickMC.cs) manages the tray icon, mouse hook and Windows actions. [SettingsWindow.cs](SettingsWindow.cs) builds the settings window. The build embeds [SidekickMC.ico](SidekickMC.ico) into the executable. The tray and settings window use the icon from that executable.

## Recognizing gestures

A low-level Windows mouse hook tracks the Back and Forward buttons and vertical wheel input. Holding one button makes the first full wheel step trigger its assigned action. The hook accumulates smaller wheel deltas until they add up to a step and consumes the scroll so the underlying app stays in place.

The hook consumes both edges of each recognized side-button press. A solo tap posts one Windows Back or Forward application command to the foreground window. A scroll gesture sends no navigation command. Pressing the second button within 300 milliseconds arms screen snipping when enabled. The app queues Snipping Tool only after both buttons are released.

## Calling Windows

- **Volume** sends volume-up or volume-down media key events. Windows adjusts the system volume and displays its own indicator.
- **Brightness** reads the active laptop panel through WMI, matches its control interface by device instance and calls `WmiSetBrightness` with a value between 0 and 100. The app requests the Windows 11 brightness flyout through the undocumented `IFlyoutDisplay.ShowFlyout(3, 0)` interface and sends the shell notification when the flyout host is present. Support can vary with the panel driver and Windows build.
- **Screen snipping** opens the Windows `ms-screenclip:` URI. Snipping Tool handles selection and capture.

## Settings and lifetime

Button assignments live in `buttons.txt` in the per-user directory returned by `Application.LocalUserAppDataPath`. Each line stores one choice, in order: Back, Forward, then the two-button action. Older two-line settings default to screen snipping for the third choice.

Closing settings hides the window. The tray menu can reopen it or exit the app. A named mutex keeps one instance running per session. Launching the executable again asks that instance to open its settings.
