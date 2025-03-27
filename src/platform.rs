///Gestisce tutte le funzionalità multipiattaforma

pub mod default_path{
    use std::path::PathBuf;

    #[cfg(target_os = "windows")]
    ///Trovo la posizione della cartella Pictures leggendo il registro di sistema
    pub fn take_default_path() -> PathBuf {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let cur_ver = hklm
            .open_subkey(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\User Shell Folders",
            )
            .expect("Errore nella lettura del registro di sistema");
        let mut pf: String = cur_ver
            .get_value("My Pictures")
            .expect("Errore nella lettura del registro di sistema");
        if pf.contains("USERPROFILE") {
            let user_profile = std::env::var("USERPROFILE").expect("Errore in USERPROFILE");
            pf = pf.replacen("%USERPROFILE%", &user_profile, 1);
        }
        PathBuf::from(pf)
    }
    #[cfg(target_os = "linux")]
    //Trovo la posizione della cartella Pictures leggendo il file user_dirs.dirs
    pub fn take_default_path() -> PathBuf {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        //Trovo la posizione della cartella .config leggendo le variabili d'ambienete
        //Se XDG_CONFIG_HOME non è definita allora è settata a default in $HOME/.config
        let home = std::env::var("HOME");

        let mut p = if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
            p
        } else {
            let mut p = home.clone().expect("Variabile d'ambiente HOME non esiste");
            p += "/.config";
            p
        };
        p += "/user_dirs.dirs";

        let f = if let Ok(f) = File::open(p) {
            let b = BufReader::new(f);
            let lines = b.lines();
            let mut ret = PathBuf::new();
            for l in lines {
                let s = l.expect("user_dirs.dirs è mal configurato");
                if s.contains("XDG_PICTURES_DIR") {
                    //prendo solo il path della cartella
                    let split = s
                        .split_once("=")
                        .expect("Variabile d'ambiente XDG_PICTURES_DIR mal configurata");
                    ret = PathBuf::from(split.1);
                    break;
                }
            }
            if ret == PathBuf::new(){
                ret = PathBuf::from(home.clone().expect("Variabile d'ambiente HOME non esiste"));
            }
            ret
        } else if let Ok(f) = home {
            PathBuf::from(f)
        } else {
            PathBuf::from("")
        };
        f
    }
    #[cfg(target_os = "macos")]
    pub fn take_default_path() -> PathBuf {
        let key = "HOME";
        let p = std::env::var(key).expect("Errore nelle variabili d'ambiente");
        PathBuf::from(p + "/pictures")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "linux")]
    fn linux_test() {
        use crate::platform;
        use std::env;
        use std::path::PathBuf;
        //Caso in cui non trova XDG_CONFIG_HOME e nemmeno il file user_dirs.dirs
        let mut key = "XDG_CONFIG_HOME";
        env::remove_var(key);
        key = "HOME";
        env::set_var(key, "tests");
        assert!(
            PathBuf::from("tests") == platform::default_path::take_default_path(),
            "Cannot handle missing user_dirs.dirs"
        );
        //Caso in cui non trova XDG_CONFIG_HOME,trova il file user_dirs.dirs ma non trova la posizione di pictures
        env::set_var(key, "tests/platform/us_dir_nok");
        assert!(
            PathBuf::from("tests/platform/us_dir_nok") == platform::default_path::take_default_path(),
            "Cannot handle missing user_dirs.dirs"
        );
        //Caso in cui non trova XDG_CONFIG_HOME
        env::set_var(key, "tests/platform/us_dir_ok");
        assert!(
            PathBuf::from("riuscito") == platform::default_path::take_default_path(),
            "Cannot handle missing XDG_CONFIG_HOME"
        );
        //Caso in cui trova XDG_CONFIG_HOME
        key = "XDG_CONFIG_HOME";
        env::set_var(key, "tests/platform/us_dir_ok/.config");
        assert!(
            PathBuf::from("riuscito") == platform::default_path::take_default_path(),
            "Cannot handle XDG_CONFIG_HOME"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_test() {
        use crate::platform;
        use std::env;
        use std::path::PathBuf;

        let key = "HOME";
        env::set_var(key, "tests");
        assert!(
            PathBuf::from("tests/pictures") == platform::default_path::take_default_path(),
            "Errore"
        );
    }
}
