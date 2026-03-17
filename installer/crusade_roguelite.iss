#define MyAppName "Crusade Roguelite"
#define MyAppPublisher "Crusade Roguelite Team"
#define MyAppExeName "crusade_roguelite.exe"
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef MyOutputVersionTag
  #define MyOutputVersionTag "0_0_0"
#endif

[Setup]
AppId={{D9D3AFD8-3D3B-4987-8748-8ED5668A641B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
VersionInfoVersion={#MyAppVersion}
VersionInfoTextVersion={#MyAppVersion}
SetupIconFile=..\assets\branding\game_icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=dist
OutputBaseFilename=crusade_roguelite_installer_{#MyOutputVersionTag}
Compression=lzma
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\data\*"; DestDir: "{app}\assets\data"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\assets\third_party\kenney_desert-shooter-pack_1.0\*"; DestDir: "{app}\assets\third_party\kenney_desert-shooter-pack_1.0"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\assets\third_party\kenney_tiny-dungeon_1.0\*"; DestDir: "{app}\assets\third_party\kenney_tiny-dungeon_1.0"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\assets\third_party\kenney_sketch-desert_1.0\*"; DestDir: "{app}\assets\third_party\kenney_sketch-desert_1.0"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
