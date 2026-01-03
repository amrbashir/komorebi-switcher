#[cfg(target_os = "windows")]
const MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<assembly manifestVersion="1.0" xmlns="urn:schemas-microsoft-com:asm.v1">
  <assemblyIdentity version="1.0.0.0" name="amrbashir.kal.app" type="win32"/>

  <dependency>
    <dependentAssembly>
      <assemblyIdentity
          type="win32"
          name="Microsoft.Windows.Common-Controls"
          version="6.0.0.0"
          processorArchitecture="*"
          publicKeyToken="6595b64144ccf1df"
          language="*"
        />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

fn main() {
    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon_with_id("assets/icon.ico", "1")
        .set_manifest(MANIFEST)
        .compile()
        .expect("Failed to compile resource file");
}
