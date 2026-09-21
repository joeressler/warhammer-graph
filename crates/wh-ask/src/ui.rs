use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui;
use wh_ask::{absolute_path, load_settings, save_settings, Citation, QueryResponse, Settings};

use crate::llama;

pub struct Prefill {
    pub bundle: Option<PathBuf>,
    pub chat_model: Option<PathBuf>,
    pub embed_model: Option<PathBuf>,
}

pub fn run(prefill: Prefill) -> Result<(), eframe::Error> {
    eframe::run_native(
        "wh-ask",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| Ok(Box::new(AskApp::new(prefill)))),
    )
}

struct AskApp {
    config_dir: PathBuf,
    bundle: String,
    chat_model: String,
    embed_model: String,
    top_k: u32,
    context_reserve: u32,
    question: String,
    answer: String,
    error: Option<String>,
    status: Option<String>,
    citations: Vec<Citation>,
    selected: Option<usize>,
    pending: Option<Receiver<Result<QueryResponse, String>>>,
}

impl AskApp {
    fn new(prefill: Prefill) -> Self {
        let config_dir = wh_ask::default_config_dir();
        let (mut settings, mut error) = match load_settings(&config_dir) {
            Ok(settings) => (settings, None),
            Err(err) => (Settings::default(), Some(err.to_string())),
        };
        let mut prefilled = false;
        if let Some(path) = prefill.bundle {
            settings.bundle = Some(absolute_path(&path).display().to_string());
            prefilled = true;
        }
        if let Some(path) = prefill.chat_model {
            settings.chat_model = Some(absolute_path(&path).display().to_string());
            prefilled = true;
        }
        if let Some(path) = prefill.embed_model {
            settings.embed_model = Some(absolute_path(&path).display().to_string());
            prefilled = true;
        }
        if prefilled {
            if let Err(err) = save_settings(&config_dir, &settings) {
                error = Some(err.to_string());
            }
        }
        AskApp {
            config_dir,
            bundle: settings.bundle.unwrap_or_default(),
            chat_model: settings.chat_model.unwrap_or_default(),
            embed_model: settings.embed_model.unwrap_or_default(),
            top_k: settings.top_k,
            context_reserve: settings.context_reserve,
            question: String::new(),
            answer: String::new(),
            error,
            status: None,
            citations: Vec::new(),
            selected: None,
            pending: None,
        }
    }

    fn persist(&mut self) {
        let settings = Settings {
            bundle: blank_to_absolute(&self.bundle),
            chat_model: blank_to_absolute(&self.chat_model),
            embed_model: blank_to_absolute(&self.embed_model),
            top_k: self.top_k.clamp(1, 32),
            context_reserve: self.context_reserve,
        };
        if let Err(err) = save_settings(&self.config_dir, &settings) {
            self.error = Some(err.to_string());
        }
    }

    fn submit(&mut self) {
        self.top_k = self.top_k.clamp(1, 32);
        self.bundle = absolutize_text(&self.bundle);
        self.chat_model = absolutize_text(&self.chat_model);
        self.embed_model = absolutize_text(&self.embed_model);
        self.persist();
        self.error = None;
        self.answer.clear();
        self.citations.clear();
        self.selected = None;
        self.status = Some(if self.embed_model.trim().is_empty() {
            "Working…".to_string()
        } else {
            "Building index…".to_string()
        });
        let bundle = self.bundle.clone();
        let chat = self.chat_model.clone();
        let embed = self.embed_model.clone();
        let question = self.question.clone();
        let top_k = self.top_k as usize;
        let reserve = self.context_reserve as usize;
        let config = self.config_dir.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let embed_path = if embed.trim().is_empty() {
                None
            } else {
                Some(PathBuf::from(embed))
            };
            let result = llama::answer_with_files(
                Path::new(&bundle),
                Path::new(&chat),
                embed_path.as_deref(),
                &question,
                top_k,
                reserve,
                &config,
            )
            .map_err(|err| err.to_string());
            let _ = tx.send(result);
        });
        self.pending = Some(rx);
    }
}

impl eframe::App for AskApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let incoming = match self.pending.as_ref().map(|pending| pending.try_recv()) {
            Some(Ok(result)) => Some(result),
            Some(Err(TryRecvError::Empty)) => {
                ctx.request_repaint();
                None
            }
            Some(Err(TryRecvError::Disconnected)) => {
                Some(Err("query stopped before it returned".to_string()))
            }
            None => None,
        };
        if let Some(result) = incoming {
            match result {
                Ok(response) => {
                    self.answer = response.answer;
                    self.citations = response.citations;
                    self.error = None;
                }
                Err(err) => {
                    self.error = Some(err);
                    self.answer.clear();
                }
            }
            self.status = None;
            self.pending = None;
        }

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.label("powered by Wahapedia");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let busy = self.pending.is_some();
            let mut dirty = false;
            ui.horizontal(|ui| {
                ui.label("Bundle");
                let response = ui.text_edit_singleline(&mut self.bundle);
                if response.lost_focus() {
                    self.bundle = absolutize_text(&self.bundle);
                    dirty = true;
                } else if response.changed() {
                    dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Chat model");
                let response = ui.text_edit_singleline(&mut self.chat_model);
                if response.lost_focus() {
                    self.chat_model = absolutize_text(&self.chat_model);
                    dirty = true;
                } else if response.changed() {
                    dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Embedding model");
                let response = ui.text_edit_singleline(&mut self.embed_model);
                if response.lost_focus() {
                    self.embed_model = absolutize_text(&self.embed_model);
                    dirty = true;
                } else if response.changed() {
                    dirty = true;
                }
            });
            if self.embed_model.trim().is_empty() {
                ui.label("Label search");
            }
            ui.horizontal(|ui| {
                ui.label("Top-k");
                if ui
                    .add(egui::DragValue::new(&mut self.top_k).range(1..=32))
                    .changed()
                {
                    dirty = true;
                }
            });
            if dirty {
                self.persist();
            }
            ui.label("Question");
            ui.add_enabled(
                !busy,
                egui::TextEdit::multiline(&mut self.question).desired_rows(3),
            );
            if ui.add_enabled(!busy, egui::Button::new("Submit")).clicked() {
                self.submit();
            }
            ui.separator();
            if let Some(error) = &self.error {
                ui.label(error);
            } else if let Some(status) = &self.status {
                ui.label(status);
            } else if !self.answer.is_empty() {
                ui.label(&self.answer);
            }
            ui.separator();
            ui.label("Citations");
            for (index, citation) in self.citations.iter().enumerate() {
                let selected = self.selected == Some(index);
                let mut line = format!("{}  {}", citation.node_id, citation.title);
                if let Some(link) = &citation.wahapedia_link {
                    line.push_str("  ");
                    line.push_str(link);
                }
                if ui.selectable_label(selected, line).clicked() {
                    self.selected = Some(index);
                }
            }
            if let Some(index) = self.selected {
                if let Some(citation) = self.citations.get(index) {
                    if citation.neighbors.is_empty() {
                        ui.label("Neighbors:");
                    } else {
                        ui.label(format!("Neighbors: {}", citation.neighbors.join(", ")));
                    }
                }
            }
        });
    }
}

fn blank_to_absolute(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(absolute_path(Path::new(trimmed)).display().to_string())
    }
}

fn absolutize_text(value: &str) -> String {
    blank_to_absolute(value).unwrap_or_default()
}
