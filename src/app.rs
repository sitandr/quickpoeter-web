use egui::{Color32, ComboBox, FontId, RichText, Slider, TextEdit};
use lazy_static::lazy_static;
use quickpoeter::{
    api::{find, string2word},
    finder::WordCollector,
    reader::{GeneralSettings, MeanStrThemes},
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    rhyme_word: String,
    #[serde(skip)]
    rhyme_output: Result<Vec<String>, String>,
    #[serde(skip)]
    show_settings: bool,
    #[serde(skip)]
    show_theme: bool,
    #[serde(skip)]
    general_settings: GeneralSettings,
    theme: Option<String>,
    value: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            rhyme_word: Default::default(),
            general_settings: Default::default(),
            show_theme: Default::default(),
            show_settings: Default::default(),
            rhyme_output: Err("Введите слово, \nк которому хотите подобрать рифму".to_owned()),
            value: 2.7,
            theme: None,
        }
    }
}

lazy_static! {
    static ref WORD_COLLECTOR: WordCollector = WordCollector::default();
    static ref MEAN_STR_THEMES: MeanStrThemes = Default::default();
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            _frame.close();
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::SidePanel::right("rhymes")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.show_theme, "Параметры рифм");
                    ui.toggle_value(&mut self.show_settings, "Параметры рифм");
                });
                ui.horizontal(|ui| {
                    let input = TextEdit::singleline(&mut self.rhyme_word).font(FontId {
                        size: 20.0,
                        family: egui::FontFamily::Monospace,
                    });

                    if ui.add(input).lost_focus() {
                        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.rhyme_output = string2word(&WORD_COLLECTOR, &self.rhyme_word)
                                .and_then(|word| {
                                    find(
                                        &WORD_COLLECTOR,
                                        &self.general_settings,
                                        word,
                                        None,
                                        &[],
                                        10,
                                    )
                                    .map(|r| r.into_iter().map(|r| r.word.src.clone()).collect())
                                })
                        };
                    }
                });

                match &self.rhyme_output {
                    Ok(res) => ui.label(RichText::new(res.join("\n")).size(18.0)),
                    Err(s) => ui.colored_label(Color32::RED, RichText::new(s).size(14.0)),
                };

                /*
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(20.0);
                    ui.toggle_value(&mut self.show_settings, "Параметры рифм");
                })
                */
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();
        });

        if self.show_settings {
            self.show_settings_window(ctx);
            self.show_theme_window(ctx);
        }
    }
}

impl TemplateApp {
    fn show_settings_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Параметры подбора рифмы").show(ctx, |ui| {
            ui.collapsing("Популярность", |ui| {
                ui.add(
                    Slider::new(&mut self.general_settings.popularity.weight, 0.0..=1e-5)
                        .clamp_to_range(false)
                        .text("Вес"),
                );

                ui.add(
                    Slider::new(&mut self.general_settings.popularity.pow, 0.0..=5.0)
                        .clamp_to_range(false)
                        .text("Степень"),
                );
            });

            if ui.button("Сбросить").clicked() {
                self.general_settings = Default::default()
            }
        });
    }

    fn show_theme_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Параметры подбора рифмы").show(ctx, |ui| {
            ComboBox::from_label("Встроенная тема")
                .selected_text(self.theme.as_ref().map(|s| s.as_str()).unwrap_or(""))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.theme, None, "Без темы");

                    for s in MEAN_STR_THEMES.str_themes.keys() {
                        ui.selectable_value(&mut self.theme, Some(s.to_string()), s);
                    }
                })
        });
    }
}
