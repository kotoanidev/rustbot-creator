; The Windows installer (D-152): for the current user, without an administrator, into
; %LOCALAPPDATA%\Programs. Unsigned in pre-alpha (D-153). Build it from the repository's
; root after a release build and THIRD-PARTY-LICENSES.txt:
;   ISCC.exe /DAppVersion=0.1.0 packaging\windows\rustbot-creator.iss

#ifndef AppVersion
  #error Pass the version: /DAppVersion=X.Y.Z
#endif

[Setup]
; Fixed forever: Windows knows the app's installs and updates by it.
AppId={{DE009DB1-48A6-499C-BC15-91778D656FDF}
AppName=RustBot Creator
AppVersion={#AppVersion}
AppVerName=RustBot Creator {#AppVersion}
AppPublisher=kotoanidev
AppPublisherURL=https://github.com/kotoanidev/rustbot-creator
AppSupportURL=https://github.com/kotoanidev/rustbot-creator/issues
DefaultDirName={localappdata}\Programs\RustBot Creator
DisableProgramGroupPage=yes
DisableDirPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\target\windows
OutputBaseFilename=rustbot-creator-{#AppVersion}-windows-x86_64-setup
SetupIconFile=rustbot-creator.ico
UninstallDisplayIcon={app}\rustbot-creator.ico
UninstallDisplayName=RustBot Creator
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "polish"; MessagesFile: "compiler:Languages\Polish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\release\rustbot-creator.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "rustbot-creator.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; DestName: "LICENSE-MIT.txt"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; DestName: "LICENSE-APACHE.txt"; Flags: ignoreversion
Source: "..\..\THIRD-PARTY-LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\RustBot Creator"; Filename: "{app}\rustbot-creator.exe"; IconFilename: "{app}\rustbot-creator.ico"; AppUserModelID: "io.github.kotoanidev.RustBotCreator"
Name: "{autodesktop}\RustBot Creator"; Filename: "{app}\rustbot-creator.exe"; IconFilename: "{app}\rustbot-creator.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\rustbot-creator.exe"; Description: "{cm:LaunchProgram,RustBot Creator}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; The hosted bots' tasks would start a program that is gone. Their data and the private
; Rust stay in %LOCALAPPDATA%\rustbot-creator.
Filename: "powershell.exe"; Parameters: "-NoProfile -NonInteractive -Command ""Get-ScheduledTask -TaskPath '\RustBot Creator\' -ErrorAction SilentlyContinue | Unregister-ScheduledTask -Confirm:$false"""; Flags: runhidden; RunOnceId: "RemoveHostedBots"
