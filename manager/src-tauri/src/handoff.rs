//! config.lua lines and the hand-off folder for a licence's server owner.

use std::path::{Path, PathBuf};

pub fn config_text(base: &str, id: u32) -> String {
    format!(
        "dllLicenseKeyFile = \"/home/SEU_USUARIO/.alurianot/dll_license.key\"\n\
         dllLicenseId = {id}\n\
         dllLicenseTokenUrl = \"{}/tokens/{id}.tok\"\n",
        base.trim_end_matches('/')
    )
}

/// <keys>\envio\<name>-<id>\ with dll_license.key + LEIA-ME.txt (CRLF, for
/// Notepad), ready to send privately to the server owner.
pub fn prepare(keys_dir: &Path, name: &str, id: u32, base: &str) -> Result<PathBuf, String> {
    let key = crate::keys::key_path(keys_dir, name, id);
    if !key.is_file() {
        return Err("A chave desta licença não está neste PC.".into());
    }
    let out = keys_dir.join("envio").join(format!("{name}-{id}"));
    std::fs::create_dir_all(&out).map_err(|e| format!("não foi possível criar {}: {e}", out.display()))?;
    std::fs::copy(&key, out.join("dll_license.key"))
        .map_err(|e| format!("não foi possível copiar a chave: {e}"))?;
    let readme = format!(
        "Licenca da ddraw.dll AlurianOT para o SEU servidor (licenca {id}).\n\
         \n\
         1. Copie dll_license.key para fora de qualquer pasta git, ex.:\n\
         \x20    mkdir -p ~/.alurianot && cp dll_license.key ~/.alurianot/ && chmod 600 ~/.alurianot/dll_license.key\n\
         2. No config.lua do servidor (troque SEU_USUARIO):\n\
         \n\
         {config}\n\
         3. Reinicie o servidor (precisa da versao do servidor com dll_license.cpp e\n\
         \x20  curl instalado). No log deve aparecer:\n\
         \x20    [DllLicense] Lease token valid until ...\n\
         \x20  O servidor precisa de internet: ele renova a licenca a cada 1 h. Sem\n\
         \x20  internet por mais de 48 h a licenca vence e o cliente para de abrir.\n\
         \n\
         Sem isso o cliente fecha sozinho ~10 s depois de entrar no jogo.\n\
         NAO compartilhe nem commite o dll_license.key.\n",
        config = config_text(base, id)
    )
    .replace('\n', "\r\n");
    std::fs::write(out.join("LEIA-ME.txt"), readme)
        .map_err(|e| format!("não foi possível gravar o LEIA-ME: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://alurianot-dll-licenses.thiago-freitasferraz.workers.dev";

    #[test]
    fn config_text_exact() {
        assert_eq!(
            config_text(BASE, 3),
            "dllLicenseKeyFile = \"/home/SEU_USUARIO/.alurianot/dll_license.key\"\n\
             dllLicenseId = 3\n\
             dllLicenseTokenUrl = \"https://alurianot-dll-licenses.thiago-freitasferraz.workers.dev/tokens/3.tok\"\n"
        );
        assert!(config_text(&format!("{BASE}/"), 3).contains(".dev/tokens/3.tok"));
    }

    #[test]
    fn prepare_writes_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("fulano-3.key"), "PEM").unwrap();
        let out = prepare(dir.path(), "fulano", 3, BASE).unwrap();
        assert_eq!(out, dir.path().join("envio").join("fulano-3"));
        assert_eq!(std::fs::read_to_string(out.join("dll_license.key")).unwrap(), "PEM");
        let readme = std::fs::read_to_string(out.join("LEIA-ME.txt")).unwrap();
        assert!(readme.contains(&config_text(BASE, 3).replace('\n', "\r\n")));
        assert!(readme.contains("Lease token valid until"));
        // Running it again refreshes the folder instead of failing.
        prepare(dir.path(), "fulano", 3, BASE).unwrap();
    }

    #[test]
    fn prepare_missing_key() {
        let dir = tempfile::tempdir().unwrap();
        let err = prepare(dir.path(), "alurianot", 1, BASE).unwrap_err();
        assert_eq!(err, "A chave desta licença não está neste PC.");
    }
}
