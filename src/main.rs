mod platform;
mod styles;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    button, column, container, pick_list, row, slider, svg, text, text_input, Canvas,
};
use iced::widget::{horizontal_rule, horizontal_space, vertical_rule, vertical_space};
use iced::{executor, widget, Application, Command, Subscription, Theme};
use iced::{theme, theme::Button, Alignment, Element, Length, Settings};
use iced::{Color, Rectangle, Size};
use image::{imageops, RgbaImage};
use pds_project::annotations::{self, Annotations};
use pds_project::color_picker::{self, ToolColor};
use pds_project::crop_tool;
use pds_project::hotkey;
use pds_project::modal::Modal;
use pds_project::screenshot;
use pds_project::{canvas_to_pixels as cp, Delays};
use pds_project::{generate_file_name, save, ImageFormat, Screen, ALL_FORMATS};
use rfd::FileDialog;
use rodio;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use styles::ButtonStyle;

//Min size crop
const MIN_SIZE_RATIO: f32 = 0.03;

#[derive(Debug, Clone)]
enum HistoryEntry {
    Annotate,
    Crop(Rectangle<u32>),
}

#[derive(Debug, Clone)]
struct ResHandles {
    crop_icon: svg::Handle,
    arrow_icon: svg::Handle,
    highlighter_icon: svg::Handle,
    display_icon: svg::Handle,
    pen_icon: svg::Handle,
    plus_icon: svg::Handle,
    square_icon: svg::Handle,
    stopwatch_icon: svg::Handle,
    text_icon: svg::Handle,
    undo_icon: svg::Handle,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PickListTools {
    #[default]
    Rectangle,
    Arrow,
    Text {
        text: String,
        size: f32,
    },
    Pen,
    Highlighter,
}

#[derive(Debug, Clone)]
pub enum Message {
    InitScreenshot,
    TakeScreenshot,
    CopyToClipboard,
    Save,
    SaveAs,
    Settings,
    ChangeFormat { format: ImageFormat },
    ToolSelected(PickListTools),
    ToolColorSelected(ToolColor),
    ScreenSelected(Screen),
    DelaySelected(Delays),
    Undo,
    ChooseSaveFolder,
    BeginCrop,
    EndCrop,
    CancelCrop,
    NewAnnotation,
}

pub fn main() -> iced::Result {
    use global_hotkey::{
        hotkey::{Code, HotKey, Modifiers},
        GlobalHotKeyManager,
    };

    // initialize the hotkeys manager
    let manager: GlobalHotKeyManager =
        GlobalHotKeyManager::new().expect("Errore nella configurazione delle hotkeys");
    // construct the hotkey
    let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);
    // register it
    manager.register(hotkey).expect("Impossibile registrare hotkey");

    let settings = Settings::<()> {
        window: iced::window::Settings {
            min_size: Some((800, 500)),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    };

    ScreenCapture::run(settings)
}

/**
path_save: path, di base o scelto dall'utente, dove salvare lo screenshot
original_screenshot: Immagine originale dello screenshot
edited_screenshot: Immagine dello screenshot modificata dall'utente
annotations: strumento per le annotazioni
history: stack delle modifiche allo screenshoot
settings: se le impostazioni devono essere mostrate a schermo o meno
format: formato dell'immagine selezionato
set_delay: se Some allora il prossimo screen avrà un delay
selected_tool: tool per le annotazioni selezionato
crop_tool: strumento per il crop
tool_color: colore selezionato da color picker
color_picker: radio button per i colori
selected_screen: su quale schermo stiamo facendo lo screen
 **/
struct ScreenCapture {
    path_save: PathBuf,
    original_screenshot: Option<RgbaImage>,
    edited_screenshot: Option<RgbaImage>,
    annotations: Annotations<Message>,
    history: Vec<HistoryEntry>,
    settings: bool,
    format: ImageFormat,
    set_delay: Option<Delays>,
    selected_tool: Option<PickListTools>,
    crop_tool: Option<crop_tool::CropTool>,
    tool_color: ToolColor,
    color_picker: color_picker::ColorPicker,
    all_screens: Vec<Screen>,
    selected_screen: Option<Screen>,
    resources: ResHandles,
}

impl ScreenCapture {
    /// Aggiorna lo stato delle annotazioni in base allo strumento e colore
    /// selezionati nella gui.
    fn update_annotations(&mut self) {
        if let Some(tool) = &self.selected_tool {
            let color = self.tool_color.into();
            let t = match tool {
                PickListTools::Rectangle => {
                    annotations::Tool::Rectangle {
                        color,
                        fill_style: annotations::FillStyle::Stroke(1.0),
                        //fill_style: annotations::FillStyle::Fill,
                    }
                }
                PickListTools::Arrow => annotations::Tool::Arrow {
                    color,
                    stroke_width: 1.0,
                },
                PickListTools::Text { text, size } => annotations::Tool::Text {
                    color,
                    content: text.clone(),
                    size: *size,
                    font: Default::default()
                },
                PickListTools::Pen => annotations::Tool::FreeHand {
                    color,
                    stroke_width: 1.0,
                },
                PickListTools::Highlighter => annotations::Tool::FreeHand {
                    color: Color { a: 0.5, ..color },
                    stroke_width: 3.0,
                },
            };
            self.annotations.set_tool(Some(t));
        } else {
            self.annotations.set_tool(None);
        }
    }

    /// Cerca nella cronologia l'ultima operazione di crop e restituisce il
    /// Rectangle che descrive la zona. Se non è mai stata fatta un'operazione
    /// di crop, retituisce None
    fn get_last_crop(&self) -> Option<Rectangle<u32>> {
        let res = self.history.iter().rfind(|h| {
            if let HistoryEntry::Crop(_) = h {
                true
            } else {
                false
            }
        });
        if let Some(HistoryEntry::Crop(ca)) = res {
            Some(*ca)
        } else {
            None
        }
    }

    /// Imposta la superficie del crop.
    fn set_screenshot_crop(&mut self, crop_rec: Option<Rectangle<u32>>) {
        if let Some(mut s) = self.original_screenshot.clone() {
            if let Some(cr) = crop_rec {
                let cropped_img = imageops::crop(&mut s, cr.x, cr.y, cr.width, cr.height);
                self.edited_screenshot = Some(cropped_img.to_image());
                self.annotations.set_crop(cr.into());
            } else {
                let cr = Rectangle {
                    x: 0,
                    y: 0,
                    width: s.width(),
                    height: s.height(),
                };
                self.edited_screenshot = Some(s);
                self.annotations.set_crop(cr.into());
            }
        }
    }

    /// Renderizza lo screenshot ritagliato e con le annotazioni.
    /// Restituisce una RgbaImage pronta per essere salvata.
    fn render_screenshot(&mut self) -> Option<RgbaImage> {
        if let Some(s) = &self.original_screenshot {
            let crop_area = self.get_last_crop();

            let ca = self.annotations.set_crop(Rectangle::with_size(Size::new(
                s.width() as f32,
                s.height() as f32,
            )));
            let tmp = cp::draw_on_buffer::<Annotations<Message>, Message>(
                &self.annotations,
                &s,
                crop_area,
            );
            self.annotations.set_crop(ca);

            tmp
        } else {
            None
        }
    }

    /// Funzione che genera la GUI della barra di selezione degli strumenti di
    /// annotazione e salvataggio.
    fn tool_selection(&self) -> Element<Message> {
        let save_buttons = row![
            button("Save")
                .style(Button::Custom(Box::new(ButtonStyle::new(
                    Color::TRANSPARENT
                ))))
                .on_press(Message::Save),
            button("Save As")
                .style(Button::Custom(Box::new(ButtonStyle::new(
                    Color::TRANSPARENT
                ))))
                .on_press(Message::SaveAs),
        ]
        .spacing(10)
        .align_items(Alignment::Center);

        let draw_controls = row![
            button(widget::svg(self.resources.pen_icon.clone()))
                .on_press(Message::ToolSelected(PickListTools::Pen))
                .style(if let Some(PickListTools::Pen) = self.selected_tool {
                    theme::Button::Primary
                } else {
                    theme::Button::Text
                }),
            button(widget::svg(self.resources.square_icon.clone()))
                .on_press(Message::ToolSelected(PickListTools::Rectangle))
                .style(if let Some(PickListTools::Rectangle) = self.selected_tool {
                    theme::Button::Primary
                } else {
                    theme::Button::Text
                }),
            button(widget::svg(self.resources.arrow_icon.clone()))
                .on_press(Message::ToolSelected(PickListTools::Arrow))
                .style(if let Some(PickListTools::Arrow) = self.selected_tool {
                    theme::Button::Primary
                } else {
                    theme::Button::Text
                }),
            button(widget::svg(self.resources.text_icon.clone()))
                .on_press(Message::ToolSelected(PickListTools::Text {
                    text: "".to_string(),
                    size: 25.0
                }))
                .style(
                    if let Some(PickListTools::Text { .. }) = self.selected_tool {
                        theme::Button::Primary
                    } else {
                        theme::Button::Text
                    }
                ),
            button(widget::svg(self.resources.highlighter_icon.clone()))
                .on_press(Message::ToolSelected(PickListTools::Highlighter))
                .style(
                    if let Some(PickListTools::Highlighter) = self.selected_tool {
                        theme::Button::Primary
                    } else {
                        theme::Button::Text
                    }
                ),
            vertical_rule(1.0),
            self.color_picker
                .view(Some(self.tool_color), |sel| -> Message {
                    Message::ToolColorSelected(sel)
                }),
        ]
        .spacing(10)
        .align_items(Alignment::Center);

        container(
            row![
                save_buttons,
                horizontal_space(Length::Fill),
                button(widget::svg(self.resources.crop_icon.clone()))
                    .on_press(Message::BeginCrop)
                    .style(theme::Button::Text),
                vertical_rule(1.0),
                draw_controls,
                horizontal_space(Length::Fill),
                if self.history.is_empty() {
                    button(widget::svg(self.resources.undo_icon.clone()))
                } else {
                    button(widget::svg(self.resources.undo_icon.clone())).on_press(Message::Undo)
                }
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .style(theme::Container::Box)
        .height(50)
        .padding(10)
        .into()
    }

    /// Funzione che genera la GUI della barra laterale di controllo
    /// dell'applicazione (nuovo screenshot, selezione delay, selezione
    /// schermo) e impostazioni.
    fn right_bar(&self) -> Element<Message> {
        let screens_pick_list = row![
            widget::svg(self.resources.display_icon.clone())
                .width(25)
                .height(25),
            pick_list(
                &self.all_screens,
                self.selected_screen,
                Message::ScreenSelected,
            )
            .placeholder("Seleziona schermo")
        ]
        .spacing(5)
        .align_items(Alignment::Center);

        let delays_pick_list = row![
            widget::svg(self.resources.stopwatch_icon.clone())
                .width(25)
                .height(25),
            pick_list(&Delays::ALL[..], self.set_delay, Message::DelaySelected)
                .placeholder("Seleziona delay"),
        ]
        .spacing(5)
        .align_items(Alignment::Center);

        let right_top_buttons = column![
            button(
                row![
                    horizontal_space(Length::Fill),
                    widget::svg(self.resources.plus_icon.clone())
                        .width(20)
                        .height(20),
                    text("New"),
                    horizontal_space(Length::Fill),
                ]
                .align_items(Alignment::Center)
                .width(Length::Fill)
            )
            .style(Button::Custom(Box::new(ButtonStyle::new(Color::WHITE))))
            .width(Length::Fill)
            .on_press(Message::InitScreenshot),
            screens_pick_list,
            delays_pick_list
        ]
        .spacing(10);
        let name = pds_project::cut_default_path(self.path_save.to_str().unwrap_or("Invalid path"));
        let right_bottom_buttons = if self.settings {
            column![
                text("Default file format:").width(Length::Fill),
                pick_list(&ALL_FORMATS[..], Some(self.format), |sel| {
                    Message::ChangeFormat { format: sel }
                })
                .placeholder("Choose a image format"),
                horizontal_rule(1.0),
                text("Default save path:").width(Length::Fill),
                button(text(name))
                    .style(Button::Custom(Box::new(ButtonStyle::new(Color::WHITE))))
                    .width(Length::Fill)
                    .on_press(Message::ChooseSaveFolder),
                button(row![
                    horizontal_space(Length::Fill),
                    text("Close settings"),
                    horizontal_space(Length::Fill),
                ])
                .style(Button::Custom(Box::new(ButtonStyle::new_with_text_color(
                    Color::TRANSPARENT,
                    Color::from_rgb(1f32, 0f32, 0f32)
                ))))
                .width(Length::Fill)
                .on_press(Message::Settings)
            ]
        } else {
            column![button(row![
                horizontal_space(Length::Fill),
                text("Settings"),
                horizontal_space(Length::Fill),
            ])
            .style(Button::Custom(Box::new(ButtonStyle::new(
                Color::TRANSPARENT
            ))))
            .width(Length::Fill)
            .on_press(Message::Settings)]
        }
        .spacing(10)
        .align_items(Alignment::Center);

        container(
            column![
                right_top_buttons,
                vertical_space(Length::FillPortion(1)),
                right_bottom_buttons
            ]
            .align_items(Alignment::Center),
        )
        .align_x(Horizontal::Right)
        .width(175)
        .height(Length::Fill)
        .style(theme::Container::Box)
        .padding(10)
        .into()
    }

    /// Funzione che genera la barra contenente i bottoni per confermare o
    /// annullare un'operazione di crop
    fn crop_dialog<'a>() -> Element<'a, Message> {
        container(
            row![
                horizontal_space(Length::Fill),
                button("Ok").on_press(Message::EndCrop),
                button("Cancel").on_press(Message::CancelCrop),
                horizontal_space(Length::Fill),
            ]
            .spacing(10),
        )
        .style(theme::Container::Box)
        .height(50)
        .padding(10)
        .into()
    }

    /// Funzione che genera la GUI per personalizzare lo strumento di
    /// annotazione Text
    fn text_dialog<'a>(text: &'a String, size: f32) -> Element<'a, Message> {
        container(
            row![
                horizontal_space(Length::Fill),
                text_input("Text...", &text)
                    .on_input(move |s| Message::ToolSelected(PickListTools::Text { text: s, size }))
                    .width(200),
                slider(1.0..=25.0, size, |v| Message::ToolSelected(
                    PickListTools::Text {
                        text: text.clone(),
                        size: v
                    }
                )),
                horizontal_space(Length::Fill),
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .style(theme::Container::Box)
        .padding(20)
        .into()
    }
}

impl Application for ScreenCapture {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flag: ()) -> (ScreenCapture, Command<Message>) {
        //Se esiste il file config, che contiene il path per il salvataggio e il formato del file, utilizza quelli altrimenti li inizializza alla cartella Pictures e Png
        let mut format = None;
        let path = if let Some(t) = pds_project::read_config_file(PathBuf::from("config.config")) {
            format = t.1;
            t.0
        } else {
            platform::default_path::take_default_path()
        };

        (
            Self {
                path_save: path,
                original_screenshot: None,
                edited_screenshot: None,
                settings: false,
                format: format.unwrap_or(ImageFormat::Png),
                set_delay: Some(Delays::default()),
                annotations: Annotations::new(iced::Size::ZERO, Message::NewAnnotation),
                history: Vec::new(),
                selected_tool: None,
                crop_tool: None,
                tool_color: color_picker::ToolColor::Black,
                color_picker: color_picker::ColorPicker {},
                all_screens: Screen::all(),
                selected_screen: Screen::primary(),
                resources: ResHandles {
                    crop_icon: svg::Handle::from_path("res/crop.svg"),
                    arrow_icon: svg::Handle::from_path("res/arrow.svg"),
                    highlighter_icon: svg::Handle::from_path("res/highlighter.svg"),
                    display_icon: svg::Handle::from_path("res/display.svg"),
                    pen_icon: svg::Handle::from_path("res/pen.svg"),
                    plus_icon: svg::Handle::from_path("res/plus.svg"),
                    square_icon: svg::Handle::from_path("res/square.svg"),
                    stopwatch_icon: svg::Handle::from_path("res/stopwatch.svg"),
                    text_icon: svg::Handle::from_path("res/text.svg"),
                    undo_icon: svg::Handle::from_path("res/undo.svg"),
                },
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Screenshot utility")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            //Salva l'immagine nella cartella di default dandogli come nome screenshot_{date}_{time}.{imageFormat} se esiste,
            //altrimenti restituisce un popup d'errore
            Message::Save => {
                if let Some(image) = self.render_screenshot() {
                    let filename = generate_file_name(self.format);

                    let mut a = self.path_save.clone();
                    a.push(filename);
                    match image.save(a.as_path()) {
                        Ok(_) => {}
                        Err(_) => {
                            pds_project::error_popup("Errore", "Salvataggio non riuscito, riprova o controlla la cartella di default");
                        }
                    }
                } else {
                    pds_project::error_popup("Nessuna immagine", "Nessuno screenshot da salvare");
                }
                Command::none()
            }
            //Apre un FileDialog per scegliere dove salvare lo screenshot e lo salva
            Message::SaveAs => {
                if let Some(image) = self.render_screenshot() {
                    let fd = FileDialog::new().clone().set_directory(&self.path_save);
                    if let Some(s) = save(fd, self.format) {
                        match image.save(s) {
                            Ok(_) => {}
                            Err(_) => {
                                pds_project::error_popup("Errore", "Salvataggio non riuscito, riprova o controlla la cartella di default");
                            }
                        }
                    }
                } else {
                    pds_project::error_popup("Nessuna immagine", "Nessuno screenshot da salvare");
                }
                Command::none()
            }

            //Riduce a icona l'applicazione, lancia un task asincrono per attendere il delay per poi inivare il messaggio TakeScreenshot
            Message::InitScreenshot => {
                let commands = [
                    iced::window::change_mode(iced::window::Mode::Hidden),
                    match self.set_delay {
                        Some(Delays::Zero) => Command::perform(
                            tokio::time::sleep(std::time::Duration::from_millis(500)),
                            |_| Message::TakeScreenshot,
                        ),
                        Some(Delays::Three) => Command::perform(
                            tokio::time::sleep(std::time::Duration::from_secs(3)),
                            |_| Message::TakeScreenshot,
                        ),
                        Some(Delays::Five) => Command::perform(
                            tokio::time::sleep(std::time::Duration::from_secs(5)),
                            |_| Message::TakeScreenshot,
                        ),
                        Some(Delays::Ten) => Command::perform(
                            tokio::time::sleep(std::time::Duration::from_secs(10)),
                            |_| Message::TakeScreenshot,
                        ),
                        _ => Command::none(),
                    },
                ];

                Command::batch(commands)
            }

            //Effettua uno screenshot sullo schermo selelzionato
            Message::TakeScreenshot => {
                //Se non ci sono scremi la unwrap chiama un panic e questo perché l'applicazione in assenza di schermi non può procedere
                if let Some(tmp) = screenshot(self.selected_screen.unwrap()) {
                    let size = iced::Size::new(tmp.width() as f32, tmp.height() as f32);
                    self.original_screenshot = Some(tmp);
                    self.edited_screenshot = self.original_screenshot.clone();

                    self.annotations.clear_annotations();
                    self.annotations.set_image_size(size);
                    self.selected_tool = None;
                    self.crop_tool = None;
                    self.history.clear();

                    'blk: {
                        // Se una qualunque di queste istruzioni fallisce è accettabile andare
                        // avanti. Verrà comunque fatto lo screenshot, ma non verrà riprodotto
                        // alcun suono.
                        let Ok((_stream, handle)) = rodio::OutputStream::try_default() else { break 'blk };
                        let Ok(sink) = rodio::Sink::try_new(&handle) else { break 'blk };

                        let Ok(file) = File::open("res/iphone-camera-capture-6448.mp3") else { break 'blk };
                        let Ok(decoder) = rodio::Decoder::new(BufReader::new(file)) else { break 'blk };

                        sink.append(decoder);
                        sink.sleep_until_end();
                    }

                    iced::window::change_mode(iced::window::Mode::Windowed)
                } else {
                    Command::none()
                }
            }
            //Se true mostra i settings
            Message::Settings => {
                self.settings = !self.settings;
                Command::none()
            }

            //Cambia il formato scelto con cui salvare l'immaggine
            Message::ChangeFormat { format } => {
                self.format = format;
                if let Ok(mut f) = File::create(PathBuf::from("config.config")) {
                    let mut s = String::from(self.path_save.to_str().unwrap());
                    s += "\n";
                    s += self.format.as_str();
                    if let Err(_) = f.write(s.as_bytes()) {
                        pds_project::error_popup(
                            "Errore",
                            "Formato cambiato ma l'opzione non è stata resa permanente",
                        );
                    }
                } else {
                    pds_project::error_popup(
                        "Errore",
                        "Formato cambiato ma l'opzione non è stata resa permanente",
                    )
                }
                Command::none()
            }

            //Cambia il tool selezionato
            Message::ToolSelected(tool) => {
                self.selected_tool = Some(tool);
                self.update_annotations();
                Command::none()
            }

            //Cambia il colore per il tool
            Message::ToolColorSelected(c) => {
                self.tool_color = c;
                self.update_annotations();
                Command::none()
            }

            Message::CopyToClipboard => {
                if let Some(image) = self.render_screenshot() {
                    let image = arboard::ImageData {
                        width: image.width() as usize,
                        height: image.height() as usize,
                        bytes: std::borrow::Cow::Borrowed(image.as_raw()),
                    };
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Err(_) = clipboard.set_image(image) {
                            pds_project::error_popup(
                                "Errore",
                                "Non è possibile gestire la clipboard",
                            );
                        }
                    } else {
                        pds_project::error_popup("Errore", "Non è possibile gestire la clipboard");
                    }
                }

                Command::none()
            }
            Message::ScreenSelected(screen) => {
                self.selected_screen = Some(screen);
                Command::none()
            }

            //Imposta il delay
            Message::DelaySelected(delay) => {
                self.set_delay = Some(delay);
                Command::none()
            }
            //Annulla l'ultima modifica sullo screenshot
            Message::Undo => {
                match self.history.pop() {
                    Some(HistoryEntry::Annotate) => {
                        self.annotations.undo_annotation();
                    }
                    Some(HistoryEntry::Crop(_)) => {
                        let crop_area = self.get_last_crop();
                        self.set_screenshot_crop(crop_area);
                    }
                    None => (),
                };
                Command::none()
            }

            //Apre una finistra per scegliere il path di default per il salvataggio delle immagini
            Message::ChooseSaveFolder => {
                let o = FileDialog::new()
                    .set_directory(&self.path_save)
                    .pick_folder();
                if let Some(path) = o {
                    self.path_save = path;
                    if let Ok(mut f) = File::create(PathBuf::from("config.config")) {
                        let mut s = String::from(self.path_save.to_str().unwrap());
                        s += "\n";
                        s += self.format.as_str();
                        if let Err(_) = f.write(s.as_bytes()) {
                            pds_project::error_popup("Errore", "Cartella di default cambiata ma non è stato possibile rendere permanente la cartella selazionata");
                        }
                    } else {
                        pds_project::error_popup("Errore", "Cartella di default cambiata ma non è stato possibile rendere permanente la cartella selazionata");
                    }
                }
                Command::none()
            }

            //Prepara l'operazione di crop
            Message::BeginCrop => {
                let Some(s) = self.render_screenshot() else {
                    pds_project::error_popup("Errore durante il rendering", "Impossibile produrre lo screenshot annotato");
                    return Command::none();
                };

                self.edited_screenshot = Some(s);

                let screenshot_size = if let Some(s) = &self.original_screenshot {
                    Rectangle {
                        x: 0,
                        y: 0,
                        width: s.width(),
                        height: s.height(),
                    }
                } else {
                    return Command::none();
                };

                let crop_rec = if let Some(cr) = self.get_last_crop() {
                    cr
                } else {
                    screenshot_size
                };

                let min_crop_size = Size::new(
                    screenshot_size.width as f32 * MIN_SIZE_RATIO,
                    screenshot_size.height as f32 * MIN_SIZE_RATIO,
                );

                self.crop_tool = Some(crop_tool::CropTool::new(crop_rec.into(), min_crop_size));
                self.selected_tool = None;
                self.update_annotations();
                Command::none()
            }

            //Sostituisce lo sreenshot con la parte selezionata
            Message::EndCrop => {
                //Se siamo qui abbiamo fatto BeginCrop e quindi crop_tool sarà sempre inizializzato
                let norm_crop_rec = self.crop_tool.as_ref().unwrap().get_crop_rec();

                if let Some(cr) = norm_crop_rec {
                    let crop_rec = cr.snap();
                    self.set_screenshot_crop(Some(crop_rec));
                    self.history.push(HistoryEntry::Crop(crop_rec));
                }

                self.crop_tool = None;
                Command::none()
            }

            //Annulla l'operazione di crop
            Message::CancelCrop => {
                let last_crop = self.get_last_crop();
                self.set_screenshot_crop(last_crop);

                self.crop_tool = None;
                Command::none()
            }

            //Aggiunge una nuova annotation alla storia delle modifiche
            Message::NewAnnotation => {
                self.history.push(HistoryEntry::Annotate);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let top_bar = if self.crop_tool.is_some() {
            Self::crop_dialog()
        } else {
            self.tool_selection()
        };

        let content = if let Some(s) = &self.edited_screenshot {
            let img = widget::image(widget::image::Handle::from_pixels(
                s.width(),
                s.height(),
                s.to_vec(),
            ));

            let tool: Element<Message> = if let Some(ct) = &self.crop_tool {
                let canvas = Canvas::new(ct).width(Length::Fill).height(Length::Fill);

                Modal::new(img, canvas).into()
            } else {
                let canvas = Canvas::new(self.annotations.clone())
                    .width(Length::Fill)
                    .height(Length::Fill);

                Modal::new(img, canvas).into()
            };

            let screenshot_canvas = container(tool)
                .padding(20)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            column![
                container(row![top_bar]).width(Length::Fill),
                container(row![
                    container(column![
                        match &self.selected_tool {
                            Some(PickListTools::Text { text, size }) => {
                                Self::text_dialog(text, *size)
                            }
                            _ => container(row![]).into(),
                        },
                        screenshot_canvas
                    ])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
                    self.right_bar()
                ])
            ]
        } else {
            let screenshot_text = text("Press SHIFT+D to take a screenshot")
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center);

            column![row![
                container(column![screenshot_text]).width(Length::Fill),
                self.right_bar(),
            ]]
        };
        container(content).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let subscriptions = [
            iced::subscription::unfold("hotkey", (), |_| hotkey::Hotkey {
                msg: Message::InitScreenshot,
            }),
            iced::subscription::events_with(|event, status| {
                if status == iced::event::Status::Captured {
                    return None;
                }

                let keyboard_event = if let iced::event::Event::Keyboard(e) = event {
                    e
                } else {
                    return None;
                };

                if let iced::keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                } = keyboard_event
                {
                    let modifier = if cfg!(target_os = "macos") {
                        iced::keyboard::Modifiers::LOGO
                    } else {
                        iced::keyboard::Modifiers::CTRL
                    };

                    if modifiers == modifier && key_code == iced::keyboard::KeyCode::C {
                        return Some(Message::CopyToClipboard);
                    } else if modifiers == modifier && key_code == iced::keyboard::KeyCode::S {
                        return Some(Message::Save);
                    }
                }
                None
            }),
        ];

        Subscription::batch(subscriptions)
    }
}
