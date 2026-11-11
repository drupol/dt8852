{
  lib,
  rustPlatform,
  versionCheckHook,
  pkg-config,
  udev,
}:

rustPlatform.buildRustPackage {
  pname = "dt8852";
  version = "0.9.2";

  __structuredAttrs = true;

  src = lib.fileset.toSource {
    root = ../../..;
    fileset = lib.fileset.unions [
      ../../../Cargo.toml
      ../../../Cargo.lock
      ../../../src
    ];
  };

  cargoHash = "sha256-9ZnNDXlkWvi06Q/HtV/mFq4posHu5ALOnZHYboh4GTo=";

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [ udev ];

  doInstallCheck = true;
  nativeInstallCheckInputs = [ versionCheckHook ];

  meta = {
    description = "A driver and command-line tool for the CEM DT-8852 sound level meter";
    homepage = "https://github.com/drupol/dt8852";
    license = lib.licenses.mit;
    mainProgram = "dt8852";
    maintainers = with lib.maintainers; [ drupol ];
    platforms = lib.platforms.all;
  };
}
