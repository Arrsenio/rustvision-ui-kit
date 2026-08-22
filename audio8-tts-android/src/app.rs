//! OLED Tron shell exposing every Audio8 TTS inference, serving, and SFT control.

use crate::android_bridge;
use crate::audio8::http::EngineEvent;
use crate::audio8::npy::CodecCodes;
use crate::audio8::params::{
    BatchRow, DType, EngineConfig, EngineKind, HealthInfo, ResponseFormat, SftConfig, SynthParams,
    SynthResult, SystemInfo, VoiceMeta,
};
use crate::audio8::text::{clean_text, supported_languages, validate_sample_id};
use crate::audio8::wav::WavClip;
use crate::audio8::HttpEngine;
use crate::theme::tron::{
    brand_title, chip, dim_text, draw_scanlines, paint_rule, paint_section, reply_text,
    section_label, tron_button_accent, tron_progress_bar, AMBER, HAIRLINE, NEON, NEON_DIM,
    OLED_BLACK, PANEL_GAP, SAFE_BOTTOM, SAFE_TOP, SCREEN_PAD, TEXT, TEXT_DIM, TOUCH_MIN,
};
use eframe::{egui, App, CreationContext, Frame};
use egui::{Button, Color32, FontId, Margin, RichText, Sense, Stroke, UiBuilder};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Synth,
    Clone,
    Batch,
    Codes,
    Engine,
    Sft,
    Archive,
}

impl Tab {
    const ALL: [Tab; 7] = [
        Tab::Synth,
        Tab::Clone,
        Tab::Batch,
        Tab::Codes,
        Tab::Engine,
        Tab::Sft,
        Tab::Archive,
    ];
    fn label(self, n: usize) -> String {
        match self {
            Tab::Synth => "SYNTH".into(),
            Tab::Clone => "CLONE".into(),
            Tab::Batch => "BATCH".into(),
            Tab::Codes => "CODES".into(),
            Tab::Engine => "ENGINE".into(),
            Tab::Sft => "SFT".into(),
            Tab::Archive => format!("ARCHIVE ({n})"),
        }
    }
}

enum JobMsg {
    Stage(String),
    Progress(f32),
    Done(Result<SynthResult, String>),
    Voices(Result<Vec<VoiceMeta>, String>),
    Health(Result<HealthInfo, String>),
    System(Result<SystemInfo, String>),
    Registered(Result<VoiceMeta, String>),
    BatchLine(String),
}

#[derive(Clone)]
struct ArchiveEntry {
    id: String,
    text: String,
    voice: String,
    model: String,
    wav: Vec<u8>,
    codes: Option<CodecCodes>,
    duration: f32,
    elapsed: f32,
    frames: usize,
    status: String,
}

pub struct TtsApp {
    tab: Tab,
    params: SynthParams,
    engine: EngineConfig,
    sft: SftConfig,
    prompt: String,
    status: String,
    error: bool,
    generating: bool,
    fraction: f32,
    stage: String,
    started: Option<Instant>,
    input_ready_at: f64,
    clip: Option<WavClip>,
    last_codes: Option<CodecCodes>,
    voices: Vec<VoiceMeta>,
    archive: Vec<ArchiveEntry>,
    batch: Vec<BatchRow>,
    batch_jsonl: String,
    health: Option<HealthInfo>,
    system: Option<SystemInfo>,
    reg_ok: Option<bool>,
    reg_reason: String,
    overwrite_voice: bool,
    voice_name: String,
    tx: Sender<JobMsg>,
    rx: Receiver<JobMsg>,
    cancel: Arc<AtomicBool>,
}

impl TtsApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();
        let params = SynthParams::default();
        Self {
            tab: Tab::Synth,
            prompt: params.text.clone(),
            params,
            engine: EngineConfig::default(),
            sft: SftConfig::default(),
            status: "POINT ENGINE AT AUDIO8 ONNX (:8024) OR SGLANG (:8010)".into(),
            error: false,
            generating: false,
            fraction: 0.0,
            stage: "IDLE".into(),
            started: None,
            input_ready_at: cc.egui_ctx.input(|i| i.time) + 1.25,
            clip: None,
            last_codes: None,
            voices: Vec::new(),
            archive: Vec::new(),
            batch: vec![BatchRow::default()],
            batch_jsonl: String::new(),
            health: None,
            system: None,
            reg_ok: None,
            reg_reason: String::new(),
            overwrite_voice: false,
            voice_name: "speaker_a".into(),
            tx,
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    fn live(&self, ui: &egui::Ui) -> bool {
        ui.input(|i| i.time) >= self.input_ready_at
    }

    fn client(&self) -> HttpEngine {
        HttpEngine::new(self.engine.clone())
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                JobMsg::Stage(s) => self.stage = s,
                JobMsg::Progress(f) => self.fraction = f,
                JobMsg::Done(res) => {
                    self.generating = false;
                    match res {
                        Ok(r) => {
                            self.error = false;
                            self.status = format!(
                                "OK · {:.2}s · {} frames · {} Hz",
                                r.elapsed_secs, r.code_frames, r.sample_rate
                            );
                            self.fraction = 1.0;
                            self.stage = if r.finished { "FINISHED" } else { "NO_EOS" }.into();
                            if !r.wav.is_empty() {
                                if let Ok(clip) = WavClip::from_wav_bytes(r.wav.clone()) {
                                    self.clip = Some(clip);
                                }
                            }
                            self.last_codes = r.codes.clone();
                            self.archive.insert(
                                0,
                                ArchiveEntry {
                                    id: self.params.sample_id.clone(),
                                    text: self.params.text.clone(),
                                    voice: self.params.voice.clone(),
                                    model: self.params.model.clone(),
                                    wav: r.wav,
                                    codes: r.codes,
                                    duration: self.clip.as_ref().map(|c| c.duration_secs()).unwrap_or(0.0),
                                    elapsed: r.elapsed_secs,
                                    frames: r.code_frames,
                                    status: self.stage.clone(),
                                },
                            );
                        }
                        Err(e) => {
                            self.error = true;
                            self.status = e;
                            self.stage = "FAIL".into();
                        }
                    }
                }
                JobMsg::Voices(res) => match res {
                    Ok(v) => {
                        self.voices = v;
                        if self.params.voice.is_empty() {
                            if let Some(first) = self.voices.first() {
                                self.params.voice = first.name.clone();
                            }
                        }
                        self.status = format!("{} VOICES", self.voices.len());
                        self.error = false;
                    }
                    Err(e) => {
                        self.error = true;
                        self.status = e;
                    }
                },
                JobMsg::Health(res) => match res {
                    Ok(h) => {
                        self.health = Some(h);
                        self.error = false;
                    }
                    Err(e) => {
                        self.error = true;
                        self.status = e;
                    }
                },
                JobMsg::System(res) => {
                    if let Ok(s) = res {
                        self.system = Some(s);
                    }
                }
                JobMsg::Registered(res) => match res {
                    Ok(v) => {
                        self.params.voice = v.name.clone();
                        self.status = format!("REGISTERED {} · {} frames", v.name, v.frames);
                        self.error = false;
                        self.refresh_voices();
                    }
                    Err(e) => {
                        self.error = true;
                        self.status = e;
                    }
                },
                JobMsg::BatchLine(s) => {
                    self.status = s;
                    self.error = false;
                }
            }
        }
        if self.generating {
            ctx.request_repaint();
        }
        if let Some((bytes, name)) = android_bridge::take_picked_audio() {
            self.params.reference_audio = Some(bytes);
            self.params.reference_audio_name = name;
            self.status = format!("REF LOADED {}", self.params.reference_audio_name);
            self.error = false;
        }
        if let Some(err) = android_bridge::picker_error() {
            self.status = err;
            self.error = true;
        }
    }

    fn spawn(&self, f: impl FnOnce(Sender<JobMsg>) + Send + 'static) {
        let tx = self.tx.clone();
        std::thread::spawn(move || f(tx));
    }

    fn speak(&mut self) {
        if self.generating {
            return;
        }
        match clean_text(&self.prompt, "text") {
            Ok(t) => self.params.text = t,
            Err(e) => {
                self.error = true;
                self.status = e;
                return;
            }
        }
        if let Err(e) = self.params.validate() {
            self.error = true;
            self.status = e;
            return;
        }
        self.generating = true;
        self.error = false;
        self.fraction = 0.08;
        self.stage = "GENERATE".into();
        self.status = "SYNTHESIZING".into();
        self.started = Some(Instant::now());
        self.cancel.store(false, Ordering::Relaxed);
        let client = self.client();
        let req = self.params.clone();
        let cancel = self.cancel.clone();
        self.spawn(move |tx| {
            let _ = tx.send(JobMsg::Stage("REQUEST".into()));
            let result = client.synthesize(&req, cancel, |ev| match ev {
                EngineEvent::Stage(s) => {
                    let _ = tx.send(JobMsg::Stage(s));
                }
                EngineEvent::Pcm { frames, .. } => {
                    let _ = tx.send(JobMsg::Progress((frames as f32 / req.max_new_tokens as f32).min(0.95)));
                    let _ = tx.send(JobMsg::Stage(format!("FRAMES {frames}")));
                }
            });
            let _ = tx.send(JobMsg::Done(result));
        });
    }

    fn refresh_voices(&self) {
        let client = self.client();
        self.spawn(move |tx| {
            let _ = tx.send(JobMsg::Voices(client.list_voices()));
        });
    }

    fn refresh_health(&self) {
        let client = self.client();
        self.spawn(move |tx| {
            let _ = tx.send(JobMsg::Health(client.health()));
            let _ = tx.send(JobMsg::System(client.system()));
        });
    }
}

impl App for TtsApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.pump(ctx);
        let live = ctx.input(|i| i.time) >= self.input_ready_at;
        let n = self.archive.len();

        egui::TopBottomPanel::top("a8_header")
            .frame(egui::Frame::NONE.fill(OLED_BLACK).inner_margin(Margin {
                left: SCREEN_PAD as i8,
                right: SCREEN_PAD as i8,
                top: SAFE_TOP as i8,
                bottom: 0,
            }))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(brand_title("AUDIO8 TTS"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let eng = if self.generating { NEON } else { NEON_DIM };
                        ui.label(chip(self.engine.kind.label(), eng));
                        ui.label(chip(
                            if self.params.voice.is_empty() {
                                "NO VOICE"
                            } else {
                                &self.params.voice
                            },
                            TEXT_DIM,
                        ));
                        ui.label(chip(&self.params.language, TEXT_DIM));
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for tab in Tab::ALL {
                        let on = self.tab == tab;
                        if tab_chip(ui, &tab.label(n), on) && live {
                            self.tab = tab;
                        }
                    }
                });
                ui.add_space(4.0);
                paint_rule(ui.painter(), ui.max_rect(), ui.cursor().top());
            });

        egui::TopBottomPanel::bottom("a8_controls")
            .frame(egui::Frame::NONE.fill(OLED_BLACK).inner_margin(Margin {
                left: SCREEN_PAD as i8,
                right: SCREEN_PAD as i8,
                top: PANEL_GAP as i8,
                bottom: SAFE_BOTTOM as i8,
            }))
            .show_separator_line(false)
            .show(ctx, |ui| {
                match self.tab {
                    Tab::Synth | Tab::Clone => self.draw_synth_controls(ui, live),
                    Tab::Archive => self.draw_archive_controls(ui, live),
                    Tab::Batch => self.draw_batch_controls(ui, live),
                    _ => {
                        ui.label(section_label("◆ ACTION"));
                        ui.label(dim_text(&self.status));
                    }
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(OLED_BLACK).inner_margin(Margin {
                left: SCREEN_PAD as i8,
                right: SCREEN_PAD as i8,
                top: PANEL_GAP as i8,
                bottom: 0,
            }))
            .show(ctx, |ui| match self.tab {
                Tab::Synth => self.draw_synth(ui),
                Tab::Clone => self.draw_clone(ui),
                Tab::Batch => self.draw_batch(ui),
                Tab::Codes => self.draw_codes(ui),
                Tab::Engine => self.draw_engine(ui, live),
                Tab::Sft => self.draw_sft(ui, live),
                Tab::Archive => self.draw_archive(ui, live),
            });
    }
}

impl TtsApp {
    fn draw_synth(&mut self, ui: &mut egui::Ui) {
        let area = ui.available_rect_before_wrap();
        let gap = PANEL_GAP;
        let wave_h = (area.height() * 0.38).clamp(120.0, 220.0);
        let wave = egui::Rect::from_min_size(area.min, egui::vec2(area.width(), wave_h));
        let rest = egui::Rect::from_min_max(
            egui::pos2(area.left(), wave.bottom() + gap),
            area.max,
        );
        let hot = if self.generating { NEON } else { NEON_DIM };
        paint_section(ui.painter(), wave, hot);
        ui.scope_builder(UiBuilder::new().id_salt("wave").max_rect(wave.shrink(10.0)), |ui| {
            ui.label(section_label("◆ WAVEFORM"));
            let inner = ui.available_rect_before_wrap();
            if let Some(clip) = &self.clip {
                draw_waveform(ui, inner, clip);
            } else {
                draw_scanlines(ui.painter(), inner);
                ui.painter().text(
                    inner.center(),
                    egui::Align2::CENTER_CENTER,
                    "NO SIGNAL",
                    FontId::monospace(13.0),
                    TEXT_DIM,
                );
            }
        });
        paint_section(ui.painter(), rest, NEON_DIM);
        ui.scope_builder(UiBuilder::new().id_salt("status").max_rect(rest.shrink(10.0)), |ui| {
            ui.label(section_label("◆ STATUS"));
            let now = ui.input(|i| i.time);
            let frac = if self.generating {
                (0.12 + (now * 0.15).fract() as f32 * 0.5).min(0.92)
            } else {
                self.fraction
            };
            tron_progress_bar(ui, frac, now);
            ui.horizontal(|ui| {
                ui.label(chip(format!("{:.0}%", frac * 100.0), NEON));
                ui.label(chip(&self.stage, TEXT_DIM));
                if let Some(t0) = self.started {
                    ui.label(chip(format!("{:.1}s", t0.elapsed().as_secs_f32()), TEXT_DIM));
                }
            });
            let color = if self.error { AMBER } else { TEXT };
            ui.add(egui::Label::new(reply_text(&self.status).color(color)).wrap());
            ui.add_space(6.0);
            ui.label(section_label("◆ SAMPLE"));
            self.draw_sample_grid(ui);
        });
    }

    fn draw_sample_grid(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("sample_grid")
            .show(ui, |ui| {
                drag_i(ui, "MAX NEW TOKENS", &mut self.params.max_new_tokens, 16..=2048);
                drag_i(ui, "RETRY MAX TOKENS", &mut self.params.retry_max_new_tokens, 16..=4096);
                drag_f(ui, "TEMPERATURE", &mut self.params.temperature, 0.05..=2.0, 0.05);
                drag_f(ui, "TOP P", &mut self.params.top_p, 0.05..=1.0, 0.01);
                drag_i(ui, "TOP K", &mut self.params.top_k, 0..=4096);
                drag_i64(ui, "SEED", &mut self.params.seed, 0..=i64::MAX / 4);
                toggle(ui, "GREEDY / ARGMAX", &mut self.params.greedy);
                toggle(ui, "STREAM", &mut self.params.stream);
                toggle(ui, "SAVE CODES", &mut self.params.save_codes);
                toggle(ui, "OVERWRITE", &mut self.params.overwrite);
                drag_i(ui, "CHUNK FRAMES", &mut self.params.chunk_frames, 1..=64);
                drag_i(ui, "STREAM CONTEXT", &mut self.params.stream_context_frames, 1..=512);
                drag_i(ui, "STREAM GUARD", &mut self.params.stream_guard_frames, 0..=8);
                ui.horizontal(|ui| {
                    ui.label(dim_text("FORMAT"));
                    for fmt in ResponseFormat::all() {
                        let on = self.params.response_format == fmt;
                        if ui
                            .add(
                                Button::new(
                                    RichText::new(format!(" {} ", fmt.as_str().to_uppercase()))
                                        .monospace()
                                        .size(11.0)
                                        .color(if on { NEON } else { TEXT_DIM }),
                                )
                                .stroke(Stroke::new(1.0, if on { NEON_DIM } else { HAIRLINE }))
                                .fill(OLED_BLACK),
                            )
                            .clicked()
                        {
                            self.params.response_format = fmt;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(dim_text("LANG"));
                    for (code, _) in supported_languages() {
                        let on = self.params.language == *code;
                        if ui
                            .add(
                                Button::new(
                                    RichText::new(format!(" {code} "))
                                        .monospace()
                                        .size(11.0)
                                        .color(if on { NEON } else { TEXT_DIM }),
                                )
                                .fill(OLED_BLACK)
                                .stroke(Stroke::new(1.0, if on { NEON_DIM } else { HAIRLINE })),
                            )
                            .clicked()
                        {
                            self.params.language = (*code).into();
                        }
                    }
                });
                labeled_edit(ui, "MODEL", &mut self.params.model);
                labeled_edit(ui, "VOICE", &mut self.params.voice);
                labeled_edit(ui, "SAMPLE ID", &mut self.params.sample_id);
            });
    }

    fn draw_synth_controls(&mut self, ui: &mut egui::Ui, live: bool) {
        ui.label(section_label("◆ PROMPT"));
        ui.add(
            egui::TextEdit::multiline(&mut self.prompt)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .font(FontId::monospace(14.0)),
        );
        let mut action = None;
        ui.columns(4, |cols| {
            let speak_on = !self.generating && !self.prompt.trim().is_empty();
            let stop_on = self.generating;
            let play_on = self.clip.is_some() && !self.generating;
            let save_on = self.clip.is_some();
            let specs = [
                ("speak", "[ SPEAK ]", speak_on, NEON),
                ("stop", "[ STOP ]", stop_on, AMBER),
                ("play", "[ PLAY ]", play_on, NEON_DIM),
                ("save", "[ SAVE ]", save_on, NEON_DIM),
            ];
            for (i, (id, label, on, accent)) in specs.into_iter().enumerate() {
                if cols[i]
                    .push_id(id, |ui| tron_button_accent(ui, label, on, accent))
                    .inner
                    .clicked()
                    && on
                    && live
                {
                    action = Some(id);
                }
            }
        });
        match action {
            Some("speak") => self.speak(),
            Some("stop") => {
                self.cancel.store(true, Ordering::Relaxed);
                let client = self.client();
                self.spawn(move |_| {
                    let _ = client.cancel();
                });
                self.generating = false;
                self.stage = "CANCEL".into();
                android_bridge::stop_playback();
            }
            Some("play") => {
                if let Some(clip) = &self.clip {
                    android_bridge::play_wav(&clip.bytes);
                }
            }
            Some("save") => {
                if let Some(clip) = &self.clip {
                    let name = format!("{}.wav", self.params.sample_id);
                    if let Some(path) = android_bridge::save_wav(&clip.bytes, &name) {
                        self.status = format!("SAVED {path}");
                        self.error = false;
                    } else {
                        self.status = "SAVE FAILED".into();
                        self.error = true;
                    }
                    if self.params.save_codes {
                        if let Some(codes) = &self.last_codes {
                            let npy = format!("{}.npy", self.params.sample_id);
                            let _ = android_bridge::save_wav(&codes.to_npy_u16(), &npy);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn draw_clone(&mut self, ui: &mut egui::Ui) {
        let area = ui.available_rect_before_wrap();
        paint_section(ui.painter(), area, NEON_DIM);
        ui.scope_builder(UiBuilder::new().id_salt("clone").max_rect(area.shrink(10.0)), |ui| {
            egui::ScrollArea::vertical().id_salt("clone_scroll").show(ui, |ui| {
                ui.label(section_label("◆ ZERO-SHOT / REGISTER"));
                ui.label(dim_text(
                    "Reference audio + exact transcript. ONNX registers a voice profile; SGLang uses references[].",
                ));
                labeled_edit(ui, "VOICE NAME", &mut self.voice_name);
                labeled_edit(ui, "REFERENCE TEXT", &mut self.params.reference_text);
                labeled_edit(ui, "SERVER AUDIO PATH", &mut self.params.reference_path);
                ui.label(dim_text(if self.params.reference_audio.is_some() {
                    format!(
                        "LOADED {} ({} bytes)",
                        self.params.reference_audio_name,
                        self.params.reference_audio.as_ref().map(|b| b.len()).unwrap_or(0)
                    )
                } else {
                    "NO LOCAL AUDIO".into()
                }));
                toggle(ui, "OVERWRITE VOICE", &mut self.overwrite_voice);
                ui.add_space(6.0);
                ui.columns(3, |cols| {
                    if cols[0]
                        .push_id("pick", |ui| tron_button_accent(ui, "[ PICK ]", true, NEON_DIM))
                        .inner
                        .clicked()
                    {
                        android_bridge::pick_audio();
                    }
                    if cols[1]
                        .push_id("reg", |ui| {
                            tron_button_accent(
                                ui,
                                "[ REGISTER ]",
                                self.params.reference_audio.is_some()
                                    && !self.params.reference_text.trim().is_empty(),
                                NEON,
                            )
                        })
                        .inner
                        .clicked()
                    {
                        if let Some(wav) = self.params.reference_audio.clone() {
                            let client = self.client();
                            let name = self.voice_name.clone();
                            let text = self.params.reference_text.clone();
                            let fname = self.params.reference_audio_name.clone();
                            let ow = self.overwrite_voice;
                            self.spawn(move |tx| {
                                let _ = tx.send(JobMsg::Registered(client.register_voice(
                                    &name, &text, &wav, &fname, ow,
                                )));
                            });
                        }
                    }
                    if cols[2]
                        .push_id("voices", |ui| tron_button_accent(ui, "[ VOICES ]", true, NEON_DIM))
                        .inner
                        .clicked()
                    {
                        self.refresh_voices();
                    }
                });
                ui.add_space(8.0);
                ui.label(section_label("◆ PROFILES"));
                if self.voices.is_empty() {
                    ui.label(dim_text("NO VOICES · REGISTER OR REFRESH"));
                }
                for v in &self.voices {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                Button::new(chip(&v.name, if v.name == self.params.voice { NEON } else { TEXT_DIM }))
                                    .fill(OLED_BLACK)
                                    .frame(false),
                            )
                            .clicked()
                        {
                            self.params.voice = v.name.clone();
                            if !v.reference_text.is_empty() {
                                self.params.reference_text = v.reference_text.clone();
                            }
                        }
                        ui.label(dim_text(format!("{}f", v.frames)));
                    });
                    if !v.reference_text.is_empty() {
                        ui.add(egui::Label::new(dim_text(&v.reference_text)).wrap());
                    }
                }
            });
        });
    }

    fn draw_batch(&mut self, ui: &mut egui::Ui) {
        let area = ui.available_rect_before_wrap();
        paint_section(ui.painter(), area, NEON_DIM);
        ui.scope_builder(UiBuilder::new().id_salt("batch").max_rect(area.shrink(10.0)), |ui| {
            ui.label(section_label("◆ JSONL MANIFEST"));
            ui.label(dim_text("Each row is an independent utterance. Same grouping rules as audio8_tts_infer.py."));
            egui::ScrollArea::vertical().id_salt("batch_scroll").show(ui, |ui| {
                let mut remove = None;
                for (i, row) in self.batch.iter_mut().enumerate() {
                    ui.add_space(6.0);
                    ui.label(chip(format!("ROW {i}"), NEON_DIM));
                    labeled_edit(ui, "ID", &mut row.id);
                    labeled_edit(ui, "TEXT", &mut row.text);
                    labeled_edit(ui, "VOICE", &mut row.voice);
                    labeled_edit(ui, "REF TEXT", &mut row.reference_text);
                    labeled_edit(ui, "REF PATH", &mut row.reference_path);
                    if ui.button(RichText::new("[ DEL ROW ]").monospace().size(11.0).color(AMBER)).clicked() {
                        remove = Some(i);
                    }
                }
                if let Some(i) = remove {
                    self.batch.remove(i);
                }
                ui.add_space(8.0);
                ui.label(section_label("◆ PASTE JSONL"));
                ui.add(
                    egui::TextEdit::multiline(&mut self.batch_jsonl)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY)
                        .font(FontId::monospace(12.0)),
                );
            });
        });
    }

    fn draw_batch_controls(&mut self, ui: &mut egui::Ui, live: bool) {
        ui.label(section_label("◆ BATCH"));
        let mut action = None;
        ui.columns(3, |cols| {
            if cols[0]
                .push_id("add", |ui| tron_button_accent(ui, "[ ADD ]", true, NEON_DIM))
                .inner
                .clicked()
                && live
            {
                action = Some("add");
            }
            if cols[1]
                .push_id("parse", |ui| {
                    tron_button_accent(ui, "[ PARSE ]", !self.batch_jsonl.trim().is_empty(), NEON_DIM)
                })
                .inner
                .clicked()
                && live
            {
                action = Some("parse");
            }
            if cols[2]
                .push_id("run", |ui| {
                    tron_button_accent(ui, "[ RUN ]", !self.generating && !self.batch.is_empty(), NEON)
                })
                .inner
                .clicked()
                && live
            {
                action = Some("run");
            }
        });
        match action {
            Some("add") => {
                let n = self.batch.len() + 1;
                self.batch.push(BatchRow {
                    id: format!("sample_{n:03}"),
                    ..BatchRow::default()
                });
            }
            Some("parse") => match parse_jsonl(&self.batch_jsonl) {
                Ok(rows) => {
                    self.batch = rows;
                    self.status = format!("PARSED {} ROWS", self.batch.len());
                    self.error = false;
                }
                Err(e) => {
                    self.error = true;
                    self.status = e;
                }
            },
            Some("run") => self.run_batch(),
            _ => {}
        }
    }

    fn run_batch(&mut self) {
        if self.generating {
            return;
        }
        self.generating = true;
        self.cancel.store(false, Ordering::Relaxed);
        let client = self.client();
        let rows = self.batch.clone();
        let mut params = self.params.clone();
        let cancel = self.cancel.clone();
        self.spawn(move |tx| {
            for row in rows {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(id) = validate_sample_id(&row.id, None) {
                    params.sample_id = id;
                }
                params.text = row.text;
                if !row.voice.is_empty() {
                    params.voice = row.voice;
                }
                params.reference_text = row.reference_text;
                params.reference_path = row.reference_path;
                let _ = tx.send(JobMsg::BatchLine(format!("RUN {}", params.sample_id)));
                let result = client.synthesize(&params, cancel.clone(), |_| {});
                let _ = tx.send(JobMsg::Done(result));
            }
        });
    }

    fn draw_codes(&mut self, ui: &mut egui::Ui) {
        let area = ui.available_rect_before_wrap();
        paint_section(ui.painter(), area, NEON_DIM);
        ui.scope_builder(UiBuilder::new().id_salt("codes").max_rect(area.shrink(10.0)), |ui| {
            ui.label(section_label("◆ CODEC INDICES  [10, T]"));
            if let Some(codes) = &self.last_codes {
                ui.label(chip(
                    format!("{} x {}  dtype=uint16  codebook=4096", codes.rows, codes.frames),
                    NEON,
                ));
                egui::ScrollArea::both().id_salt("code_grid").show(ui, |ui| {
                    let show_t = codes.frames.min(24);
                    for r in 0..codes.rows {
                        ui.horizontal(|ui| {
                            ui.label(chip(format!("CB{r:02}"), NEON_DIM));
                            for t in 0..show_t {
                                ui.label(
                                    RichText::new(format!("{:4}", codes.at(r, t)))
                                        .monospace()
                                        .size(10.0)
                                        .color(TEXT),
                                );
                            }
                            if codes.frames > show_t {
                                ui.label(dim_text("…"));
                            }
                        });
                    }
                });
                if ui.button(RichText::new("[ COPY NPY HEX LEN ]").monospace().size(11.0).color(NEON_DIM)).clicked() {
                    android_bridge::copy_text(&format!("frames={} bytes={}", codes.frames, codes.to_npy_u16().len()));
                }
            } else {
                draw_scanlines(ui.painter(), ui.available_rect_before_wrap());
                ui.label(dim_text("NO CODES · ENABLE SAVE CODES AND SYNTHESIZE"));
            }
        });
    }

    fn draw_engine(&mut self, ui: &mut egui::Ui, live: bool) {
        let area = ui.available_rect_before_wrap();
        paint_section(ui.painter(), area, NEON_DIM);
        ui.scope_builder(UiBuilder::new().id_salt("engine").max_rect(area.shrink(10.0)), |ui| {
            egui::ScrollArea::vertical().id_salt("engine_scroll").show(ui, |ui| {
                ui.label(section_label("◆ BACKEND"));
                ui.horizontal(|ui| {
                    for kind in EngineKind::all() {
                        let on = self.engine.kind == kind;
                        if ui
                            .add(
                                Button::new(
                                    RichText::new(format!(" {} ", kind.label()))
                                        .monospace()
                                        .size(11.0)
                                        .color(if on { NEON } else { TEXT_DIM }),
                                )
                                .stroke(Stroke::new(1.0, if on { NEON_DIM } else { HAIRLINE }))
                                .fill(OLED_BLACK),
                            )
                            .clicked()
                            && live
                        {
                            self.engine.kind = kind;
                            self.engine.base_url = match kind {
                                EngineKind::OnnxHttp => "http://10.0.2.2:8024".into(),
                                EngineKind::Sglang => "http://10.0.2.2:8010".into(),
                                EngineKind::LocalOnnx => String::new(),
                            };
                        }
                    }
                });
                labeled_edit(ui, "BASE URL", &mut self.engine.base_url);
                labeled_edit(ui, "HF MODEL", &mut self.engine.hf_model);
                labeled_edit(ui, "MODEL DIR", &mut self.engine.model_dir);
                labeled_edit(ui, "VOICES DIR", &mut self.engine.voices_dir);
                labeled_edit(ui, "REGISTRATION DIR", &mut self.engine.registration_dir);
                labeled_edit(ui, "PRECISION", &mut self.engine.precision);
                labeled_edit(ui, "CODEC PRECISION", &mut self.engine.codec_precision);
                labeled_edit(ui, "DEVICE", &mut self.engine.device);
                ui.horizontal(|ui| {
                    ui.label(dim_text("DTYPE"));
                    for dt in DType::all() {
                        let on = self.engine.dtype == dt;
                        if ui
                            .add(
                                Button::new(
                                    RichText::new(format!(" {} ", dt.as_str()))
                                        .monospace()
                                        .size(11.0)
                                        .color(if on { NEON } else { TEXT_DIM }),
                                )
                                .fill(OLED_BLACK)
                                .stroke(Stroke::new(1.0, if on { NEON_DIM } else { HAIRLINE })),
                            )
                            .clicked()
                        {
                            self.engine.dtype = dt;
                        }
                    }
                });
                drag_i(ui, "THREADS", &mut self.engine.threads, 1..=16);
                let mut timeout = self.engine.timeout_secs as i32;
                drag_i(ui, "TIMEOUT S", &mut timeout, 5..=600);
                self.engine.timeout_secs = timeout.max(5) as u64;
                ui.add_space(8.0);
                ui.columns(3, |cols| {
                    if cols[0]
                        .push_id("health", |ui| tron_button_accent(ui, "[ HEALTH ]", true, NEON_DIM))
                        .inner
                        .clicked()
                        && live
                    {
                        self.refresh_health();
                    }
                    if cols[1]
                        .push_id("voices2", |ui| tron_button_accent(ui, "[ VOICES ]", true, NEON_DIM))
                        .inner
                        .clicked()
                        && live
                    {
                        self.refresh_voices();
                    }
                    if cols[2]
                        .push_id("reload", |ui| tron_button_accent(ui, "[ RELOAD ]", true, AMBER))
                        .inner
                        .clicked()
                        && live
                    {
                        let client = self.client();
                        self.spawn(move |tx| {
                            match client.reload() {
                                Ok(s) => {
                                    let _ = tx.send(JobMsg::BatchLine(s));
                                }
                                Err(e) => {
                                    let _ = tx.send(JobMsg::Done(Err(e)));
                                }
                            }
                        });
                    }
                });
                if let Some(h) = &self.health {
                    ui.label(chip(
                        format!(
                            "DUALAR {} · CODEC {} · {}",
                            h.precision.to_uppercase(),
                            h.codec_precision.to_uppercase(),
                            h.provider
                        ),
                        NEON,
                    ));
                }
                if let Some(s) = &self.system {
                    ui.label(dim_text(format!(
                        "MEM {:.0} / peak {:.0} MB · up {:.0}s",
                        s.current_mb.unwrap_or(0.0),
                        s.peak_mb.unwrap_or(0.0),
                        s.uptime_seconds
                    )));
                }
                ui.label(dim_text(
                    "LOCAL DUALAR: Rust generate loop is in audio8::dualar (slow AR, fast AR, Gumbel, EOS, codebook packing). Wire ort sessions onto DualArBackend to run fully on-device.",
                ));
            });
        });
    }

    fn draw_sft(&mut self, ui: &mut egui::Ui, live: bool) {
        let area = ui.available_rect_before_wrap();
        paint_section(ui.painter(), area, NEON_DIM);
        ui.scope_builder(UiBuilder::new().id_salt("sft").max_rect(area.shrink(10.0)), |ui| {
            egui::ScrollArea::vertical().id_salt("sft_scroll").show(ui, |ui| {
                ui.label(section_label("◆ SUPERVISED FINE-TUNE"));
                ui.label(dim_text(
                    "Training stays off-device (torch.distributed). This panel owns every audio8_tts_sft.sh / ModelArguments / DataArguments / Audio8TTSTrainingArguments knob and exports the command.",
                ));
                labeled_edit(ui, "MODEL", &mut self.sft.model_name_or_path);
                labeled_edit(ui, "TRAIN JSONL", &mut self.sft.train_jsonl);
                labeled_edit(ui, "EVAL JSONL", &mut self.sft.eval_jsonl);
                labeled_edit(ui, "OUTPUT DIR", &mut self.sft.output_dir);
                labeled_edit(ui, "EXPORT DIR", &mut self.sft.export_dir);
                labeled_edit(ui, "RESUME", &mut self.sft.resume_mode);
                labeled_edit(ui, "LR SCHEDULER", &mut self.sft.lr_scheduler_type);
                labeled_edit(ui, "REPORT TO", &mut self.sft.report_to);
                labeled_edit(ui, "MASTER ADDR", &mut self.sft.master_addr);
                let mut max_len = self.sft.max_length as i32;
                drag_i(ui, "MAX LENGTH", &mut max_len, 64..=8192);
                self.sft.max_length = max_len as u32;
                let mut bs = self.sft.batch_size as i32;
                drag_i(ui, "BATCH", &mut bs, 1..=64);
                self.sft.batch_size = bs as u32;
                let mut ebs = self.sft.eval_batch_size as i32;
                drag_i(ui, "EVAL BATCH", &mut ebs, 1..=64);
                self.sft.eval_batch_size = ebs as u32;
                let mut ga = self.sft.gradient_accumulation_steps as i32;
                drag_i(ui, "GRAD ACCUM", &mut ga, 1..=128);
                self.sft.gradient_accumulation_steps = ga as u32;
                drag_f64(ui, "LEARNING RATE", &mut self.sft.learning_rate, 1e-8..=1e-2, 1e-6);
                drag_f(ui, "EPOCHS", &mut self.sft.num_train_epochs, 0.1..=100.0, 0.1);
                drag_f(ui, "WARMUP RATIO", &mut self.sft.warmup_ratio, 0.0..=1.0, 0.001);
                drag_f(ui, "WEIGHT DECAY", &mut self.sft.weight_decay, 0.0..=0.3, 0.001);
                drag_f(ui, "MAX GRAD NORM", &mut self.sft.max_grad_norm, 0.1..=10.0, 0.1);
                drag_f(ui, "SLOW LOSS W", &mut self.sft.slow_loss_weight, 0.0..=10.0, 0.1);
                drag_f(ui, "FAST LOSS W", &mut self.sft.fast_loss_weight, 0.0..=10.0, 0.1);
                toggle(ui, "FREEZE SLOW AR", &mut self.sft.freeze_slow_ar);
                toggle(ui, "FREEZE FAST AR", &mut self.sft.freeze_fast_ar);
                toggle(ui, "BF16", &mut self.sft.bf16);
                toggle(ui, "GRAD CHECKPOINT", &mut self.sft.gradient_checkpointing);
                let mut nn = self.sft.nnodes as i32;
                drag_i(ui, "NNODES", &mut nn, 1..=64);
                self.sft.nnodes = nn as u32;
                let mut np = self.sft.nproc_per_node as i32;
                drag_i(ui, "NPROC", &mut np, 1..=16);
                self.sft.nproc_per_node = np as u32;
                let mut port = self.sft.master_port as i32;
                drag_i(ui, "MASTER PORT", &mut port, 1..=65535);
                self.sft.master_port = port as u32;
                ui.add_space(8.0);
                if ui
                    .push_id("copy_sft", |ui| tron_button_accent(ui, "[ COPY COMMAND ]", true, NEON))
                    .inner
                    .clicked()
                    && live
                {
                    android_bridge::copy_text(&self.sft.to_shell());
                    ctx_copy(ui, &self.sft.to_shell());
                    self.status = "SFT COMMAND COPIED".into();
                }
                if ui
                    .push_id("copy_json", |ui| tron_button_accent(ui, "[ COPY JSON ]", true, NEON_DIM))
                    .inner
                    .clicked()
                    && live
                {
                    android_bridge::copy_text(&self.sft.to_pretty_json());
                    ctx_copy(ui, &self.sft.to_pretty_json());
                }
                ui.add(
                    egui::Label::new(dim_text(self.sft.to_shell())).wrap().selectable(true),
                );
            });
        });
    }

    fn draw_archive(&mut self, ui: &mut egui::Ui, live: bool) {
        let area = ui.available_rect_before_wrap();
        paint_section(ui.painter(), area, NEON_DIM);
        ui.scope_builder(UiBuilder::new().id_salt("archive").max_rect(area.shrink(10.0)), |ui| {
            ui.label(section_label("◆ ARCHIVE"));
            if self.archive.is_empty() {
                draw_scanlines(ui.painter(), ui.available_rect_before_wrap());
                ui.centered_and_justified(|ui| ui.label(dim_text("NO ENTRIES")));
                return;
            }
            let mut play = None;
            let mut del = None;
            let mut copy = None;
            egui::ScrollArea::vertical().id_salt("arch_list").show(ui, |ui| {
                for (i, e) in self.archive.iter().enumerate() {
                    egui::Frame::NONE
                        .stroke(Stroke::new(1.0, HAIRLINE))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(chip(&e.status, NEON_DIM));
                                ui.label(chip(&e.voice, TEXT_DIM));
                                ui.label(chip(format!("{:.1}s", e.duration), TEXT_DIM));
                                ui.label(chip(format!("{}f", e.frames), TEXT_DIM));
                                if ui
                                    .add(Button::new(RichText::new("[ PLAY ]").monospace().size(11.0).color(NEON)).frame(false))
                                    .clicked()
                                    && live
                                {
                                    play = Some(i);
                                }
                                if ui
                                    .add(Button::new(RichText::new("[ COPY ]").monospace().size(11.0).color(TEXT_DIM)).frame(false))
                                    .clicked()
                                    && live
                                {
                                    copy = Some(i);
                                }
                                if ui
                                    .add(Button::new(RichText::new("[ DEL ]").monospace().size(11.0).color(AMBER)).frame(false))
                                    .clicked()
                                    && live
                                {
                                    del = Some(i);
                                }
                            });
                            ui.add(egui::Label::new(reply_text(&e.text)).wrap().selectable(true));
                            ui.label(dim_text(format!("{} · {}", e.id, e.model)));
                        });
                    ui.add_space(6.0);
                }
            });
            if let Some(i) = play {
                android_bridge::play_wav(&self.archive[i].wav);
                if !self.archive[i].wav.is_empty() {
                    if let Ok(clip) = WavClip::from_wav_bytes(self.archive[i].wav.clone()) {
                        self.clip = Some(clip);
                    }
                }
                self.last_codes = self.archive[i].codes.clone();
            }
            if let Some(i) = copy {
                android_bridge::copy_text(&self.archive[i].text);
                ctx_copy(ui, &self.archive[i].text);
            }
            if let Some(i) = del {
                self.archive.remove(i);
            }
        });
    }

    fn draw_archive_controls(&mut self, ui: &mut egui::Ui, live: bool) {
        ui.label(section_label("◆ ARCHIVE"));
        let on = !self.archive.is_empty();
        if ui
            .push_id("clear", |ui| tron_button_accent(ui, "[ CLEAR ALL ]", on, AMBER))
            .inner
            .clicked()
            && on
            && live
        {
            self.archive.clear();
            self.clip = None;
            self.last_codes = None;
        }
    }
}

fn tab_chip(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let color = if active { NEON } else { TEXT_DIM };
    let stroke = Stroke::new(1.0, if active { NEON_DIM } else { HAIRLINE });
    ui.add(
        Button::new(RichText::new(format!(" {label} ")).monospace().size(12.0).color(color))
            .stroke(stroke)
            .fill(OLED_BLACK),
    )
    .clicked()
}

fn labeled_edit(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(dim_text(label));
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .font(FontId::monospace(13.0)),
    );
}

fn drag_i(ui: &mut egui::Ui, label: &str, value: &mut i32, range: std::ops::RangeInclusive<i32>) {
    ui.horizontal(|ui| {
        ui.label(dim_text(label));
        ui.add(egui::DragValue::new(value).range(range).speed(1.0));
    });
}

fn drag_i64(ui: &mut egui::Ui, label: &str, value: &mut i64, range: std::ops::RangeInclusive<i64>) {
    ui.horizontal(|ui| {
        ui.label(dim_text(label));
        ui.add(egui::DragValue::new(value).range(range).speed(1.0));
    });
}

fn drag_f(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, speed: f64) {
    ui.horizontal(|ui| {
        ui.label(dim_text(label));
        ui.add(egui::DragValue::new(value).range(range).speed(speed).max_decimals(4));
    });
}

fn drag_f64(ui: &mut egui::Ui, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>, speed: f64) {
    ui.horizontal(|ui| {
        ui.label(dim_text(label));
        ui.add(egui::DragValue::new(value).range(range).speed(speed).max_decimals(8));
    });
}

fn toggle(ui: &mut egui::Ui, label: &str, on: &mut bool) {
    let text = format!("{} {}", if *on { "[x]" } else { "[ ]" }, label);
    if ui
        .add(
            Button::new(
                RichText::new(text)
                    .monospace()
                    .size(12.0)
                    .color(if *on { NEON } else { TEXT_DIM }),
            )
            .fill(OLED_BLACK)
            .stroke(Stroke::new(1.0, if *on { NEON_DIM } else { HAIRLINE })),
        )
        .clicked()
    {
        *on = !*on;
    }
}

fn draw_waveform(ui: &mut egui::Ui, rect: egui::Rect, clip: &WavClip) {
    let painter = ui.painter_at(rect);
    let n = ((rect.width() / 3.0) as usize).clamp(32, 160);
    let peaks = clip.peaks(n);
    let w = rect.width() / n as f32;
    let mid = rect.center().y;
    let h = rect.height() * 0.42;
    for (i, p) in peaks.iter().enumerate() {
        let x = rect.left() + i as f32 * w;
        let mag = (*p).clamp(0.02, 1.0) * h;
        painter.line_segment(
            [egui::pos2(x, mid - mag), egui::pos2(x, mid + mag)],
            Stroke::new(1.2, NEON_DIM),
        );
    }
}

fn parse_jsonl(src: &str) -> Result<Vec<BatchRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in src.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("line {}: {e}", i + 1))?;
        rows.push(BatchRow {
            id: v.get("id").and_then(|x| x.as_str()).unwrap_or("").into(),
            text: v.get("text").and_then(|x| x.as_str()).unwrap_or("").into(),
            voice: v
                .get("voice")
                .or_else(|| v.get("voice_name"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
            reference_text: v
                .get("reference_text")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
            reference_path: v
                .get("reference_audio")
                .or_else(|| v.get("audio_path"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
        });
    }
    if rows.is_empty() {
        return Err("no inference rows found".into());
    }
    Ok(rows)
}

fn ctx_copy(ui: &mut egui::Ui, text: &str) {
    ui.ctx().copy_text(text.to_string());
}

#[allow(dead_code)]
fn _touch() -> f32 {
    TOUCH_MIN
}
