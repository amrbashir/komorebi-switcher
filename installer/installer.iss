[Setup]
AppId=komorebi-switcher
AppName=komorebi-switcher
AppVersion=0.10.4
AppPublisher=Amr Bashir
AppPublisherURL=https://github.com/amrbashir/komorebi-switcher
AppCopyright=Copyright 2025 Amr Bashir. Licensed under the MIT License.
DefaultDirName={code:GetDefaultDir}
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=.
OutputBaseFilename=komorebi-switcher-setup
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\komorebi-switcher.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern dynamic windows11
DisableProgramGroupPage=yes

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"

[Files]
Source: "komorebi-switcher.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\komorebi-switcher"; Filename: "{app}\komorebi-switcher.exe"
Name: "{userdesktop}\komorebi-switcher"; Filename: "{app}\komorebi-switcher.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\komorebi-switcher.exe"; Description: "Launch komorebi-switcher"; Flags: nowait postinstall skipifsilent

[Code]
const
  LegacyUninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\komorebi-switcher';

function GetDefaultDir(Param: String): String;
begin
  // TODO: Remove this legacy lookup once upgrades from NSIS are no longer supported.
  if RegQueryStringValue(HKCU, LegacyUninstallKey, 'InstallLocation', Result) then
    Result := RemoveQuotes(Result)
  else
    Result := ExpandConstant('{localappdata}\komorebi-switcher');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  LegacyDir, InstallDir: String;
begin
  // TODO: Remove this legacy lookup once upgrades from NSIS are no longer supported.
  // Retire NSIS only after successfully installing into the same directory.
  if CurStep <> ssPostInstall then
    Exit;

  if not RegQueryStringValue(HKCU, LegacyUninstallKey, 'InstallLocation', LegacyDir) then
    Exit;

  LegacyDir := RemoveBackslashUnlessRoot(RemoveQuotes(LegacyDir));
  InstallDir := RemoveBackslashUnlessRoot(ExpandConstant('{app}'));
  if CompareText(LegacyDir, InstallDir) <> 0 then
    Exit;

  DeleteFile(ExpandConstant('{app}\uninstall.exe'));
  RegDeleteKeyIncludingSubkeys(HKCU, LegacyUninstallKey);
end;

// Keep the minimum version in sync with Reactor's native/bootstrap.rs.
const
  RuntimeFamily = 'Microsoft.WindowsAppRuntime.2_8wekyb3d8bbwe';
  RuntimeMinVersion = '2.6.4';
  RuntimeMinVersionPacked = $0002000600040000;
  RuntimeUrl = 'https://aka.ms/windowsappsdk/2.6/2.6.5/windowsappruntimeinstall-x64.exe';
  RuntimeSHA256 = '908e9a8ed8ad2ac765d6679543b0758812437e3ad6e3223f08d1730635cec8ec';

function TryCreatePackageDependency(User: NativeInt; Family: String;
  MinVersion: Int64; Architectures: Cardinal; LifetimeKind: Integer;
  LifetimeArtifact: NativeInt; Options: Cardinal; var DependencyId: NativeInt): Integer;
  external 'TryCreatePackageDependency@kernelbase.dll stdcall delayload';
function DeletePackageDependency(DependencyId: NativeInt): Integer;
  external 'DeletePackageDependency@kernelbase.dll stdcall delayload';
function ProcessHeap(): THandle;
  external 'GetProcessHeap@kernel32.dll stdcall';
function FreeHeapMemory(Heap: THandle; Flags: Cardinal; Memory: NativeInt): Boolean;
  external 'HeapFree@kernel32.dll stdcall';

var
  RuntimeDownloadPage: TDownloadWizardPage;

function WindowsAppRuntimeInstalled(): Boolean;
var
  DependencyId: NativeInt;
begin
  Result := False;
  DependencyId := 0;
  try
    try
      // Resolve an x64 or neutral package for the current user, as Reactor does.
      Result := TryCreatePackageDependency(0, RuntimeFamily, RuntimeMinVersionPacked, 5, 0, 0, 0, DependencyId) = 0;
      if Result then
        DeletePackageDependency(DependencyId);
    finally
      if DependencyId <> 0 then
        FreeHeapMemory(ProcessHeap(), 0, DependencyId);
    end;
  except
    Result := False;
    Log('Could not resolve Windows App Runtime: ' + GetExceptionMessage);
  end;
end;

procedure InitializeWizard();
begin
  RuntimeDownloadPage := CreateDownloadPage('Windows App Runtime',
    'Downloading the runtime required by komorebi-switcher...', nil);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Installer: String;
  ExitCode: Integer;
begin
  Result := '';

  if WindowsAppRuntimeInstalled() then
    Exit;

  Installer := ExpandConstant('{tmp}\WindowsAppRuntimeInstall.exe');
  try
    try
      RuntimeDownloadPage.Clear;
      RuntimeDownloadPage.Add(RuntimeUrl, 'WindowsAppRuntimeInstall.exe', RuntimeSHA256);
      RuntimeDownloadPage.Show;
      try
        RuntimeDownloadPage.Download;
      finally
        RuntimeDownloadPage.Hide;
      end;

      if not Exec(Installer, '--quiet', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
        RaiseException('Could not start the runtime installer: ' + SysErrorMessage(ExitCode));
      Log(Format('Runtime installer exit code: %d', [ExitCode]));
      if not WindowsAppRuntimeInstalled() then
        RaiseException(Format('The required runtime is still unavailable (exit code %d).', [ExitCode]));
    finally
      DeleteFile(Installer);
    end;
  except
    Result := 'Could not install Windows App Runtime ' + RuntimeMinVersion +
      ' or newer. Check your internet connection, or install the x64 runtime from Microsoft and retry.' + #13#10 +
      GetExceptionMessage;
  end;
end;
