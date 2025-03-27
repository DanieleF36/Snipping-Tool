pub mod annotations;
pub mod canvas_to_pixels;
pub mod color_picker;
pub mod crop_tool;
pub mod hotkey;
pub mod modal;

use image::RgbaImage;
use screenshots;
use std::fmt::Formatter;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ImageFormat {
    Png,
    Bmp,
    Jpeg,
    Gif,
}
pub const ALL_FORMATS: [ImageFormat; 4] = [
    ImageFormat::Png,
    ImageFormat::Bmp,
    ImageFormat::Jpeg,
    ImageFormat::Gif,
];

impl ImageFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Png => "Png",
            Self::Bmp => "Bitmap",
            Self::Jpeg => "Jpeg",
            Self::Gif => "Gif",
        }
    }
}

impl Into<image::ImageFormat> for ImageFormat {
    fn into(self) -> image::ImageFormat {
        match self {
            Self::Png => image::ImageFormat::Png,
            Self::Bmp => image::ImageFormat::Bmp,
            Self::Jpeg => image::ImageFormat::Jpeg,
            Self::Gif => image::ImageFormat::Gif,
        }
    }
}

impl From<image::ImageFormat> for ImageFormat {
    fn from(value: image::ImageFormat) -> Self {
        match value {
            image::ImageFormat::Png => Self::Png,
            image::ImageFormat::Bmp => Self::Bmp,
            image::ImageFormat::Jpeg => Self::Jpeg,
            image::ImageFormat::Gif => Self::Gif,
            _ => Self::Png,
        }
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ImageFormat::Png => "Png",
                ImageFormat::Bmp => "Bitmap",
                ImageFormat::Jpeg => "Jpeg",
                ImageFormat::Gif => "Gif",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Delays {
    #[default]
    Zero,
    Three,
    Five,
    Ten,
}

impl Delays {
    pub const ALL: [Delays; 4] = [Delays::Zero, Delays::Three, Delays::Five, Delays::Ten];
}

impl std::fmt::Display for Delays {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Delays::Zero => "Nessun delay",
                Delays::Three => "3s",
                Delays::Five => "5s",
                Delays::Ten => "10s",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Screen {
    id: u32,
    index: usize,
    primary: bool,
}

impl Screen {
    pub fn all() -> Vec<Self> {
        let screens = screenshots::Screen::all().unwrap();
        screens
            .iter()
            .enumerate()
            .map(|(i, s)| Screen {
                id: s.display_info.id,
                index: i + 1,
                primary: s.display_info.is_primary,
            })
            .collect()
    }

    pub fn primary() -> Option<Self> {
        let screens = screenshots::Screen::all().unwrap();
        screens.iter().enumerate().find_map(|(i, s)| {
            if s.display_info.is_primary {
                let ret = Screen {
                    id: s.display_info.id,
                    index: i + 1,
                    primary: s.display_info.is_primary,
                };
                Some(ret)
            } else {
                None
            }
        })
    }
}

impl std::fmt::Display for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.primary {
            write!(f, "{} (primary)", self.index)
        } else {
            write!(f, "{}", self.index)
        }
    }
}

pub fn screenshot(screen: Screen) -> Option<RgbaImage> {
    let screens = screenshots::Screen::all()
        .expect("Non esistono schermi e qundi non si può lanciare l'applicazione");
    let screen = screens.iter().find(|s| s.display_info.id == screen.id);

    let screen = if let Some(s) = screen {
        s
    } else {
        //Se superiamo il primo expect allora c'è sicuramente un primo schermo
        screens.first().unwrap()
    };

    if let Ok(image) = screen.capture() {
        let buffer = image.buffer();
        let ret: RgbaImage = image::load_from_memory(buffer).expect("TODO").to_rgba8();
        Some(ret)
    } else {
        error_popup(
            "Errore",
            "Non è stato possibile fare lo screenshot, riprova",
        );
        None
    }
}

use chrono::{DateTime, Local, Utc};
use rfd::FileDialog;
///Genera il nome dello screenshot secondo il seguente pattern: screenshot_{date}_{time}.{imageFormat}
pub fn generate_file_name(format: ImageFormat) -> String {
    let utc = Utc::now();
    let converted: DateTime<Local> = DateTime::from(utc);

    // Ottieni la data e l'ora correnti
    let date = converted.format("%Y-%m-%d").to_string();
    let time = converted.format("%H-%M-%S").to_string();

    let extension = Into::<image::ImageFormat>::into(format).extensions_str()[0];
    format!("screenshot_{}_{}.{}", date, time, extension)
}

///Apre un FileDialog per far scegliere dove salvare e che nome dare allo screenshot, impostandolo prima a uno di default
///Restituisce il path scelto dall'utente
pub fn save(task: FileDialog, format: ImageFormat) -> Option<PathBuf> {
    let filename = generate_file_name(format);

    let mut path = task.set_file_name(&*filename);

    let mut formats = ALL_FORMATS;
    let pos = formats.iter().position(|f| format == *f).unwrap();
    formats.swap(0, pos);

    for f in formats {
        let ext_str = Into::<image::ImageFormat>::into(f).extensions_str();
        path = path.add_filter(f.as_str(), ext_str);
    }
    let path = path.save_file();
    path
}
///Mostra un popup di errore con titolo e descrizione passati come argomenti
pub fn error_popup(title: &str, description: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(description)
        .show();
}

///Legge il file di configurazione
pub fn read_config_file(path: PathBuf) -> Option<(PathBuf, Option<ImageFormat>)> {
    let mut f = if let Ok(f) = File::open(path) {
        f
    } else {
        return None;
    };
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("Non è stato possibile leggere il file");
    let mut l = s.lines();
    let path = if let Some(p) = l.next() {
        p
    } else {
        return None;
    };
    let m = if let Some(m) = l.next() {
        m
    } else {
        return None;
    };
    let format = match m {
        "Png" => Some(ImageFormat::Png),
        "Bmp" => Some(ImageFormat::Bmp),
        "Jpeg" => Some(ImageFormat::Jpeg),
        "Gif" => Some(ImageFormat::Gif),
        _ => None,
    };
    Some((PathBuf::from(path), format))
}
///Tronca la stringa in modo tale da non farla andare a capo a una dimensione fissa
pub fn cut_default_path(s: &str) -> String {
    if s.len() > 21 {
        let split = s.split_at(21 - 3);
        let mut ret = String::from(split.0);
        ret.push_str("...");
        ret
    } else {
        String::from(s)
    }
}
#[cfg(test)]
mod test {
    use crate::{cut_default_path, read_config_file, ImageFormat};
    use std::path::PathBuf;

    #[test]
    fn read_config_file_test() {
        let ret = read_config_file(PathBuf::from("tests/config.config")).unwrap();
        assert!(ret.0 == PathBuf::from("path") && ret.1.unwrap() == ImageFormat::Png)
    }
    #[test]
    fn cut_default_path_test() {
        let s = "abcdefghijklmnopqr...".to_string();
        let a = cut_default_path("abcdefghijklmnopqrstuw");

        assert!(a == s, "Errore");
    }
}
