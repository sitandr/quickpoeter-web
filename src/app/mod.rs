use egui::{Color32, ComboBox, FontId, RichText, Slider, TextEdit, Ui};
use lazy_static::lazy_static;
use quickpoeter::{
    api::{find, string2word},
    finder::WordCollector,
    meaner::MeanTheme,
    reader::{GeneralSettings, MeanStrThemes},
};

mod highlighter;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct QuickpoeterApp {
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

    custom_theme_text: String,
    theme: Theme,
    rps: RemovePartsOfSpeech,
    show_rhymes: u32,
    main_text: String,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct RemovePartsOfSpeech {
    /// с      существительное
    /// п      прилагательное
    /// мс     местоимение-существительное
    /// мс-п   местоименное-прилагательное
    /// г      глагол
    /// н      наречие
    /// числ   числительное
    /// числ-п счётное прилагательное
    /// вводн  вводное слово
    /// межд   межометие
    /// предик предикатив
    /// предл  предлог
    /// союз   союз
    /// сравн  сравнительная степень
    /// част   частица
    /// ?      куски фразеологизмов и т.п.
    noun: bool,
    adj: bool,
    pronoun: bool,
    pronoun_adj: bool,
    verb: bool,
    adv: bool,
    num: bool,
    num_adj: bool,
    linking: bool,
    citoslovce: bool,
    pred: bool,
    prep: bool,
    conj: bool,
    compare: bool,
    part: bool,
    misc: bool,
}

impl RemovePartsOfSpeech {
    fn get_list(&self) -> Vec<&'static str> {
        let mut v = vec![];

        macro_rules! add {
            ($flag: expr, $name: expr) => {
                if $flag {
                    v.push($name)
                }
            };
        }

        add!(self.noun, "с");
        add!(self.adj, "п");
        add!(self.pronoun, "мс");
        add!(self.pronoun_adj, "мс-п");
        add!(self.verb, "г");
        add!(self.adv, "н");
        add!(self.num, "числ");
        add!(self.num_adj, "числ-п");
        add!(self.linking, "вводн");
        add!(self.citoslovce, "межд");
        add!(self.pred, "предик");
        add!(self.prep, "предл");
        add!(self.conj, "союз");
        add!(self.compare, "сравн");
        add!(self.part, "част");
        add!(self.misc, "?");
        v
    }
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq)]
enum Theme {
    No,
    Preset(String),
    Custom,
}

impl From<Option<String>> for Theme {
    fn from(preset: Option<String>) -> Self {
        preset.map_or(Self::No, Self::Preset)
    }
}

impl Theme {
    fn name(&self) -> String {
        match self {
            Self::No => "Без темы".to_string(),
            Self::Preset(s) => s.clone(),
            Self::Custom => "Пользовательская".to_string(),
        }
    }

    fn mean_theme(&self, custom_theme_text: &str) -> Result<Option<MeanTheme>, Vec<String>> {
        let splitted;
        let words = match self {
            Self::No => return Ok(None),
            Self::Preset(s) => &MEAN_STR_THEMES.str_themes[s],
            Self::Custom => {
                splitted = custom_theme_text
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect();
                &splitted
            }
        };

        MeanTheme::from_str(&WORD_COLLECTOR, words)
            .map(Some)
            .map_err(|v| v.into_iter().cloned().collect())
    }
}

impl Default for QuickpoeterApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            main_text: String::new(),
            rhyme_word: String::new(),
            general_settings: GeneralSettings::default(),
            show_theme: Default::default(),
            show_settings: Default::default(),
            rhyme_output: Ok(vec![]),
            rps: RemovePartsOfSpeech::default(),
            custom_theme_text: String::new(),
            show_rhymes: 50,
            theme: Theme::No,
        }
    }
}

lazy_static! {
    static ref WORD_COLLECTOR: WordCollector = WordCollector::default();
    static ref MEAN_STR_THEMES: MeanStrThemes = MeanStrThemes::default();
}

impl QuickpoeterApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Self::default()
    }
}

impl eframe::App for QuickpoeterApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
                    ui.toggle_value(&mut self.show_theme, "Тема");
                    ui.toggle_value(&mut self.show_settings, "Параметры рифм");
                });

                if self.show_theme {
                    self.show_theme_select(ui);
                }

                ui.horizontal(|ui| {
                    let input = TextEdit::singleline(&mut self.rhyme_word)
                        .font(FontId {
                            size: 20.0,
                            family: egui::FontFamily::Monospace,
                        })
                        .hint_text("К чему рифму?");

                    let response = ui.add_sized(ui.available_size(), input);

                    if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.rhyme_output = string2word(&WORD_COLLECTOR, &self.rhyme_word)
                            .and_then(|word| {
                                find(
                                    &WORD_COLLECTOR,
                                    &self.general_settings,
                                    word,
                                    self.theme
                                        .mean_theme(&self.custom_theme_text)
                                        .map_err(|err| match err.len() {
                                            0 => "Пустая тема".to_string(),
                                            _ => format!("Неизвестные слова: {err:?}"),
                                        })?
                                        .as_ref(),
                                    &self.rps.get_list(),
                                    self.show_rhymes,
                                )
                                .map(|r| r.into_iter().map(|r| r.word.src.clone()).collect())
                            });
                    }
                });

                match &self.rhyme_output {
                    Ok(res) => {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| ui.label(RichText::new(res.join("\n")).size(18.0)));
                    }
                    Err(s) => {
                        ui.colored_label(Color32::RED, RichText::new(s).size(14.0));
                    }
                };

                /*
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(20.0);
                    ui.toggle_value(&mut self.show_settings, "Параметры рифм");
                })
                */
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_sized(
                ui.available_size(),
                TextEdit::multiline(&mut self.main_text)
                    .code_editor()
                    .font(FontId {
                        size: 20.0,
                        family: egui::FontFamily::Monospace,
                    }),
            )
        });

        self.show_settings_window(ctx);
    }
}

impl QuickpoeterApp {
    fn show_settings_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Параметры подбора рифмы").open(&mut self.show_settings).show(ctx, |ui| {

            macro_rules! default_or {
                ($default: expr) => {
                    $default
                };
                ($default: expr, $some: expr) => {
                    $some
                };
            }

            ui.add(
                Slider::new(&mut self.show_rhymes, 1..=500)
                    .text("Количество отображаемых рифм")
            );

            if ui.button("Сбросить").clicked() {
                self.general_settings = GeneralSettings::default();
            }

            ui.checkbox(&mut self.general_settings.stresses.indexation, "Индексация гласных");

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    macro_rules! setting {
                        ($block: expr => {$($name: expr, $mut_ref: expr, $max: expr $(, $min: expr)?;)*}) => {
                            ui.collapsing($block, |ui| {
                                $(
                                    ui.add(
                                        Slider::new(&mut $mut_ref, default_or!(0.0$(, $min)? )..=$max)
                                            .clamp_to_range(false)
                                            .text($name),
                                    );
                                )*
                            })
                        };
                    }

                    setting!("Веса" => {
                        "Тематика", self.general_settings.meaning.weight, 5000.0;
                        "Популярность слова", self.general_settings.popularity.weight, 1e-5;
                        "Ударения", self.general_settings.stresses.weight, 200.0;
                        "Структура", self.general_settings.consonant_structure.weight, 10.0;
                        "Аллитерации", self.general_settings.alliteration.weight, 10.0;
                    });

                    setting!("Тематика" => {
                        "Степень", self.general_settings.meaning.pow, 5.0;
                        "Степень для одного слова", self.general_settings.meaning.single_pow, 5.0;
                        "Множитель для одного слова", self.general_settings.meaning.single_weight, 1.0;
                        "Вес", self.general_settings.meaning.weight, 5000.0;
                    });

                    setting!("Разное" => {
                        "Близкая длина", self.general_settings.misc.length_diff_fine, 3.0;
                        "Совпадающие гласные/согласные в конце", self.general_settings.misc.same_cons_end, 3.0;
                    });

                    setting!("Популярность слова" => {
                        "Вес", self.general_settings.popularity.weight, 1e-5;
                        "Степень", self.general_settings.popularity.pow, 5.0;
                    });

                    setting!("Ударения" => {
                        "Строгие ударения", self.general_settings.stresses.k_strict_stress, 40.0;
                        "Нестрогие ударения", self.general_settings.stresses.k_not_strict_stress, 7.0;
                        "Штраф за плохой ритм", self.general_settings.stresses.bad_rythm, 100.0;
                        "Сдвиг веса сравнения гласных", self.general_settings.stresses.shift_syll_ending, 3.0;
                        "Степень веса сравнения гласных", self.general_settings.stresses.pow_syll_ending, 3.0;
                        "Ассимптотика метрики", self.general_settings.stresses.asympt, 3.0;
                        "Сдвиг ассимптотики метрики", self.general_settings.stresses.asympt_shift, 2.0;
                        "Вес", self.general_settings.stresses.weight, 200.0;
                    });

                    setting!("Длина искомой рифмы" => {
                        "Идеальная длина", self.general_settings.unsymmetrical.optimal_length, 15.0;
                        "Вес штрафа меньших", self.general_settings.unsymmetrical.less_w, 0.5;
                        "Степень штрафа меньших", self.general_settings.unsymmetrical.less_pow, 1.1;
                        "Вес штрафа больших", self.general_settings.unsymmetrical.more_w, 0.5;
                        "Степень штрафа больших", self.general_settings.unsymmetrical.more_pow, 1.1;
                    });

                    setting!("Штрафы за совпадающие части речи" => {
                        "Глаголы", self.general_settings.same_speech_part.verb, 2.0;
                        "Прилагательные", self.general_settings.same_speech_part.adj, 1.0;
                        "Существительные", self.general_settings.same_speech_part.noun, 1.0;
                        "Наречия", self.general_settings.same_speech_part.adv, 1.0;
                    });

                    setting!("Структура" => {
                        "Степень разности длин слогов", self.general_settings.consonant_structure.pow, 5.0;
                        "Сдвиг множителя сравнения с конца", self.general_settings.consonant_structure.shift_syll_ending, 5.0;
                        "Степень множителя сравнения с конца", self.general_settings.consonant_structure.pow_syll_ending, 5.0;
                        "Ассимптотика метрики", self.general_settings.consonant_structure.asympt, 3.0;
                        "Сдвиг ассимптотики", self.general_settings.consonant_structure.asympt_shift, 5.0;
                        "Вес", self.general_settings.consonant_structure.weight, 10.0;
                    });

                    setting!("Аллитерации" => {
                        "Сдвиг расстояния в слове между буквами", self.general_settings.alliteration.shift_coord, 5.0;
                        "Степень расстояния в слове между буквами", self.general_settings.alliteration.pow_coord_delta, 5.0;
                        "Сдвиг важности согласных в концовке", self.general_settings.alliteration.shift_syll_ending, 5.0;
                        "Степень важности согласных в концовке", self.general_settings.alliteration.pow_syll_ending, 3.0, -3.0;
                        "Штраф за дополнительные звуки", self.general_settings.alliteration.permutations, 50.0;
                        "Ассимптотика метрики", self.general_settings.alliteration.asympt, 3.0;
                        "Сдвиг ассимптотики", self.general_settings.alliteration.asympt_shift, 5.0;
                        "Вес", self.general_settings.alliteration.weight, 10.0;
                    });

                    ui.collapsing("Исключить части речи", |ui| {
                        ui.checkbox(&mut self.rps.noun, "Существительные");
                        ui.checkbox(&mut self.rps.adj, "Прилагательные");
                        ui.checkbox(&mut self.rps.pronoun, "Местоимения");
                        ui.checkbox(&mut self.rps.pronoun_adj, "Местоимения-прилагательные");
                        ui.checkbox(&mut self.rps.verb, "Глаголы");
                        ui.checkbox(&mut self.rps.adv, "Наречия");
                        ui.checkbox(&mut self.rps.num, "Числительные");
                        ui.checkbox(&mut self.rps.num_adj, "Счётные прилагательные");
                        ui.checkbox(&mut self.rps.linking, "Вводные");
                        ui.checkbox(&mut self.rps.citoslovce, "Междометия");
                        ui.checkbox(&mut self.rps.pred, "Предикативы");
                        ui.checkbox(&mut self.rps.prep, "Предлоги");
                        ui.checkbox(&mut self.rps.conj, "Союзы");
                        ui.checkbox(&mut self.rps.compare, "Сравнительные степени");
                        ui.checkbox(&mut self.rps.part, "Частицы");
                        ui.checkbox(&mut self.rps.misc, "Прочее (фразеологизмы, устаревшие…)");
                    });
                }
            )
        });
    }

    fn show_theme_select(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ComboBox::from_label("Встроенная тема")
            .selected_text(self.theme.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.theme, Theme::No, "Без темы");

                for s in MEAN_STR_THEMES.str_themes.keys() {
                    ui.selectable_value(&mut self.theme, Theme::Preset(s.to_string()), s);
                }
            });
        ui.selectable_value(&mut self.theme, Theme::Custom, "Пользовательская");

        if self.theme == Theme::Custom {
            ui.add(
                TextEdit::multiline(&mut self.custom_theme_text)
                    .hint_text("Введите слова, ассоциирующиеся с этой темой"),
            );
        }
        ui.add_space(10.0);
    }
}
