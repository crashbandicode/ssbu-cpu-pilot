//! Bounded, once-per-results UI policy. One step is one UI input-builder frame.
//! Native observations are supplied by the Switch adapter; no game calls here.
use crate::settings::Settings;

pub const A: u32 = 1 << 5;
pub const Y: u32 = 1 << 7;
pub const RIGHT: u32 = 1 << 21;
pub const SAVE_RECORD: u64 = 0x58ff_ff14_5ea6_f7be;
pub const SUCCESS_RECORD: u64 = 0x58ff_ff1e_1eed_5764;
pub const REMATCH_RECORD: u64 = 0x58ff_ff1c_b2b1_1288;
/// Offline Cards wait, counted from the last splash A (or epoch begin when none
/// was pressed), so the splash presses below never eat into it.
pub const CARDS_TIMEOUT: u16 = 1200;
/// Offline results open on the winner splash, which shows no cards and never
/// advances alone (Eden c17: unchanged for 28 s; c14: the cards appeared only
/// after A presses). First A this many frames after epoch begin, then one every
/// `SPLASH_GAP` frames, at most `SPLASH_MAX` per epoch, and none once any cards
/// sample (ready or not) for the epoch exists. Eden c18: three presses at
/// +2.3..4.3 s all landed during the loading gear and the winner's intro; the
/// splash only accepted input from about +5 s, so the presses keep coming
/// (about once a second) for long enough to cover a slow intro.
pub const SPLASH_SETTLE: u16 = 90;
pub const SPLASH_GAP: u16 = 60;
pub const SPLASH_MAX: u8 = 12;
/// Offline, before the cards sample there is no fighter-entry port mapping. The
/// splash A goes to UI port 0 (P1's controller; Eden c19). Online the splash A
/// rides the local entry's mapped port, like the rest of the save.
pub const SPLASH_PORT: u8 = 0;
pub const DIALOG_TIMEOUT: u16 = 180;
pub const SAVE_TIMEOUT: u16 = 900;
pub const CLOSE_TIMEOUT: u16 = 480;
pub const ANSWER_TIMEOUT: u16 = 360;
/// Online, from epoch begin to a ready "Rematch?" pop-up. Eden r01
/// (2026-10-31): the winner splash ran about 15 s before the pop-up, while
/// 480 frames lasted about 8.5 s, so the step expired before the pop-up. The
/// pop-up's own countdown is about 10 s; 2400 frames (about 40 s at the
/// measured callback rate) covers the splash and the countdown.
pub const REMATCH_WAIT: u16 = 2400;
pub const REMATCH_SETTLE: u32 = 36;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    Offline,
    Online,
    Unsupported,
}

/// Training (`melee 0x0d`) shares the game mode `0x0201xxxx` with local Smash.
pub const MELEE_TRAINING: u8 = 0x0d;
/// Online battles (`melee 0x0e`).
pub const MELEE_ONLINE: u8 = 0x0e;
/// Replay playback (with melee 0).
pub const MODE_PLAYBACK: u32 = 0x0501_0100;

/// Local Smash is the `0x0201xxxx` mode family outside Training: Eden c14
/// (2026-10-31) read `0x02010000` at local Smash results. Online families
/// (`0x07xxxxxx` or melee `0x0e`) are online; everything else, replay
/// playback and Training included, is unsupported and never pressed.
pub fn scene(initialized: bool, melee: u8, mode: u32) -> Scene {
    if !initialized {
        return Scene::Unsupported;
    }
    if melee == MELEE_ONLINE || mode >> 24 == 7 {
        return Scene::Online;
    }
    if melee == MELEE_TRAINING || mode == MODE_PLAYBACK {
        return Scene::Unsupported;
    }
    match mode & 0xffff_0000 {
        0x0201_0000 => Scene::Offline,
        _ => Scene::Unsupported,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Popup {
    pub record: u64,
    pub phase: i32,
    pub pending: i32,
    pub age: u32,
    pub result: u32,
    pub decided: bool,
    pub opening: bool,
    pub default_button: u32,
    pub cancel: u32,
    pub input_mode: u32,
    pub cursor: i32,
}

impl Popup {
    pub fn ready(self) -> bool {
        self.phase == 3
            && self.pending == 3
            && self.age >= 1
            && self.result == 0
            && !self.decided
            && !self.opening
    }
    fn ordinary(self) -> bool {
        self.record == REMATCH_RECORD
            && self.default_button == 2
            && self.cancel == 0
            && self.input_mode == 2
    }
    fn save(self) -> bool {
        self.record == SAVE_RECORD
            && self.default_button == 1
            && self.cancel == 0
            && self.input_mode == 2
    }
    fn expired(self) -> bool {
        self.phase >= 5 || self.pending >= 6 || self.decided || self.result != 0
    }
}

/// epoch is the live results-sequence identity, zero when absent. A zero sample
/// between two matches disambiguates a reused native address.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub epoch: usize,
    pub scene: Scene,
    pub port: Option<u8>,
    /// Any cards sample (ready or not) was taken for this epoch.
    pub cards_seen: bool,
    pub cards_ready: bool,
    pub save_status: Option<i32>,
    pub popup: Option<Popup>,
    /// Results identity captured by the shared ordinary-rematch parse callback.
    pub rematch_epoch: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub held: u32,
    pub pressed: u32,
    pub released: u32,
    pub right: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Record {
    pub held: u32,
    pub pressed: u32,
    pub released: u32,
    pub pad: u32,
    pub left: [f32; 4],
    pub right: [f32; 4],
}
const _: () = assert!(core::mem::size_of::<Record>() == 0x30);

impl Input {
    /// Overlay only this port. Native records are rebuilt every frame; never
    /// clear physical input or synthesize input for an unbound port.
    pub fn apply(self, records: &mut [Record; 10], port: u8) {
        let Some(r) = records.get_mut(usize::from(port)) else {
            return;
        };
        r.held |= self.held;
        r.pressed |= self.pressed;
        r.released |= self.released;
        if self.right {
            r.left[0] = 1.0;
            r.left[1] = 0.0;
            r.left[3] = 0.0;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Begin,
    /// Online: the "Rematch?" pop-up was parsed before the save's Y could be
    /// pressed, so the save is given up and the rematch is answered.
    SaveLate,
    Unsupported,
    /// The k-th (1-based) splash A of this epoch, on `SPLASH_PORT`.
    SplashA(u8),
    SaveY,
    SaveRight,
    SaveConfirm,
    Saving,
    SaveClose,
    Saved,
    RematchA,
    RematchYes,
    Timeout,
    FailedStatus,
    UnexpectedStatus,
    LostPopup,
    WrongAnswer,
    Cancelled,
    SceneChanged,
    PortChanged,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub input: Input,
    pub port: Option<u8>,
    pub event: Option<Event>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Cards,
    SaveDialog,
    SelectYes,
    Confirm,
    Saving,
    Close,
    Rematch,
    Answer,
    End,
}

/// Fixed button pulse: one edge, five holds, one release, then ten neutral
/// frames. A lost signal cancels immediately, with only a release remaining.
#[derive(Clone, Copy, Debug)]
struct Pulse {
    mask: u32,
    at: u8,
}
impl Pulse {
    const fn new() -> Self {
        Self { mask: 0, at: 17 }
    }
    fn start(&mut self, mask: u32) {
        self.mask = mask;
        self.at = 0;
    }
    fn idle(self) -> bool {
        self.at >= 17
    }
    fn cancel(&mut self) -> Input {
        let release = if self.at > 0 && self.at <= 6 {
            self.mask
        } else {
            0
        };
        *self = Self::new();
        Input {
            released: release,
            ..Input::default()
        }
    }
    fn step(&mut self) -> Input {
        let n = self.at;
        self.at = self.at.saturating_add(1).min(17);
        Input {
            held: if n < 6 { self.mask } else { 0 },
            pressed: if n == 0 { self.mask } else { 0 },
            released: if n == 6 { self.mask } else { 0 },
            right: false,
        }
    }
}

pub struct Results {
    epoch: usize,
    stage: Stage,
    age: u16,
    /// Frames since epoch begin: the online rematch deadline counts from here,
    /// however long the save before it took.
    epoch_age: u16,
    scene: Scene,
    port: Option<u8>,
    save: bool,
    rematch: bool,
    pulse: Pulse,
    right_frames: u8,
    neutral: u8,
    rematch_seen: bool,
    /// Latched: a cards sample existed in this epoch. Ends the splash presses.
    cards_seen: bool,
    splash: u8,
    /// The in-flight pulse is a splash A, which rides `splash_port`, not `port`.
    splash_pulse: bool,
    splash_port: u8,
}

impl Default for Results {
    fn default() -> Self {
        Self::new()
    }
}

impl Results {
    pub const fn new() -> Self {
        Self {
            epoch: 0,
            stage: Stage::Idle,
            age: 0,
            epoch_age: 0,
            scene: Scene::Unsupported,
            port: None,
            save: false,
            rematch: false,
            pulse: Pulse::new(),
            right_frames: 0,
            neutral: 0,
            rematch_seen: false,
            cards_seen: false,
            splash: 0,
            splash_pulse: false,
            splash_port: SPLASH_PORT,
        }
    }
    pub fn stage(&self) -> Stage {
        self.stage
    }
    fn go(&mut self, stage: Stage) {
        self.stage = stage;
        self.age = 0;
    }
    /// Port carrying the in-flight pulse. Read it before cancelling or stepping.
    fn lane(&self) -> Option<u8> {
        if self.splash_pulse && !self.pulse.idle() {
            Some(self.splash_port)
        } else {
            self.port
        }
    }
    fn end(&mut self, event: Event) -> Output {
        self.go(Stage::End);
        let port = self.lane();
        Output {
            input: self.pulse.cancel(),
            port,
            event: Some(event),
        }
    }
    /// The stages of the save, from the wait for the cards to the success
    /// pop-up's close.
    fn saving(&self) -> bool {
        matches!(
            self.stage,
            Stage::Cards | Stage::SaveDialog | Stage::SelectYes | Stage::Confirm | Stage::Saving | Stage::Close
        )
    }
    /// After the save (completed, failed or given up): online with Auto
    /// rematch on, the epoch goes on to wait for the "Rematch?" pop-up;
    /// otherwise it ends.
    fn next(&mut self, event: Event) -> Output {
        if self.scene != Scene::Online || !self.rematch {
            return self.end(event);
        }
        self.go(Stage::Rematch);
        self.splash_pulse = false;
        let port = self.lane();
        Output {
            input: self.pulse.cancel(),
            port,
            event: Some(event),
        }
    }
    fn press(&mut self, mask: u32, stage: Stage, event: Event) -> Output {
        self.go(stage);
        self.splash_pulse = false;
        self.pulse.start(mask);
        Output {
            input: self.pulse.step(),
            port: self.port,
            event: Some(event),
        }
    }
    /// A on the winner splash, before any cards sample, on `port`. Restarts the
    /// Cards deadline so the presses never eat into it.
    fn press_splash(&mut self, port: u8) -> Output {
        self.go(Stage::Cards);
        self.splash = self.splash.saturating_add(1);
        self.splash_pulse = true;
        self.splash_port = port;
        self.pulse.start(A);
        Output {
            input: self.pulse.step(),
            port: Some(port),
            event: Some(Event::SplashA(self.splash)),
        }
    }
    /// The next splash A is due: still waiting for the first cards sample, under
    /// the press cap, the settle delay or the gap since the last press elapsed,
    /// and no pulse in flight.
    fn splash_due(&self) -> bool {
        let due = if self.splash == 0 {
            SPLASH_SETTLE
        } else {
            SPLASH_GAP
        };
        self.stage == Stage::Cards
            && !self.cards_seen
            && self.splash < SPLASH_MAX
            && self.age >= due
            && self.pulse.idle()
    }
    fn wait(&mut self) -> Output {
        let port = self.lane();
        Output {
            input: self.pulse.step(),
            port,
            event: None,
        }
    }
    pub fn step(&mut self, f: Frame, settings: Settings) -> Output {
        if f.epoch != self.epoch {
            let interrupted = self.epoch != 0 && !matches!(self.stage, Stage::End | Stage::Idle);
            let old_port = self.lane();
            let release = self.pulse.cancel();
            *self = Self {
                epoch: f.epoch,
                scene: f.scene,
                port: f.port.filter(|p| *p < 10),
                save: settings.auto_save_replay,
                rematch: settings.auto_rematch,
                ..Self::new()
            };
            if f.epoch != 0 {
                // Online the save runs first: the game shows "Rematch?" about
                // 21 s after the match (Eden r02), the save chain takes about
                // 1.2 s (Eden c19), and `next` moves on to the rematch after it.
                self.stage = match f.scene {
                    Scene::Offline | Scene::Online if self.save => Stage::Cards,
                    Scene::Online if self.rematch => Stage::Rematch,
                    _ => Stage::End,
                };
            }
            return Output {
                input: release,
                port: old_port,
                event: if interrupted {
                    Some(Event::SceneChanged)
                } else if f.epoch != 0 {
                    Some(Event::Begin)
                } else {
                    None
                },
            };
        }
        if self.epoch == 0 || self.stage == Stage::End {
            return Output::default();
        }
        if f.scene != self.scene {
            return self.end(Event::SceneChanged);
        }
        // A row turned off mid-epoch cancels its part: the save gives way to
        // the rematch (online, when that row is on), the rematch just ends.
        let saving = self.saving();
        if saving && !settings.auto_save_replay {
            return self.next(Event::Cancelled);
        }
        if !saving && !settings.auto_rematch {
            return self.end(Event::Cancelled);
        }
        // Bind once when the controller mapping becomes available.
        if self.port.is_none() {
            self.port = f.port.filter(|p| *p < 10);
        }
        if self.port.is_some() && f.port != self.port {
            return self.end(Event::PortChanged);
        }
        // The first cards sample of the epoch ends the splash presses for good.
        // Stop holding an A still in flight: its release goes to its own port.
        let first_sample = f.cards_seen && !self.cards_seen;
        self.cards_seen |= f.cards_seen;
        // A pop-up opening or shown (online the "Rematch?" one comes over the
        // splash) also stops a splash A in flight: the A must never reach it.
        let popup_busy = f.popup.is_some_and(|p| p.phase != 0 || p.opening);
        if (first_sample || popup_busy) && self.splash_pulse && !self.pulse.idle() {
            let port = self.lane();
            return Output {
                input: self.pulse.cancel(),
                port,
                event: None,
            };
        }
        self.age = self.age.saturating_add(1);
        self.epoch_age = self.epoch_age.saturating_add(1);
        // Deadline wins even when the expected signal arrives on that frame.
        // The rematch's counts from epoch begin, whatever the save took.
        let expired = match self.stage {
            Stage::Cards => self.age >= CARDS_TIMEOUT,
            Stage::SaveDialog | Stage::SelectYes | Stage::Confirm => self.age >= DIALOG_TIMEOUT,
            Stage::Saving => self.age >= SAVE_TIMEOUT,
            Stage::Close => self.age >= CLOSE_TIMEOUT,
            Stage::Rematch => self.epoch_age >= REMATCH_WAIT,
            Stage::Answer => self.age >= ANSWER_TIMEOUT,
            _ => true,
        };
        if expired {
            return if saving {
                self.next(Event::Timeout)
            } else {
                self.end(Event::Timeout)
            };
        }
        if self.port.is_none() {
            // Offline, no cards sample yet: the winner splash is on screen.
            if self.scene == Scene::Offline && self.splash_due() {
                return self.press_splash(SPLASH_PORT);
            }
            return self.wait();
        }
        if saving && f.save_status == Some(4) {
            return self.next(Event::FailedStatus);
        }
        let p = f.popup;
        match self.stage {
            // Online, the "Rematch?" pop-up parsed before Y was pressed: the
            // game has moved on, so the save is given up for the rematch.
            Stage::Cards if self.scene == Scene::Online && f.rematch_epoch == self.epoch => {
                return self.next(Event::SaveLate);
            }
            Stage::Cards if f.cards_ready && f.save_status == Some(0) => {
                return self.press(Y, Stage::SaveDialog, Event::SaveY);
            }
            // Online the winner splash counts down to "Rematch?" and shows no
            // cards unless A is pressed on it (Eden s02), as offline (c17–c19).
            // The A rides the mapped port, never while a pop-up opens or shows.
            Stage::Cards if self.scene == Scene::Online && !popup_busy && self.splash_due() => {
                if let Some(port) = self.port {
                    return self.press_splash(port);
                }
            }
            Stage::SaveDialog => {
                if f.save_status.is_some_and(|s| s != 0 && s != 1) {
                    return self.next(Event::UnexpectedStatus);
                }
                if f.save_status == Some(1) && self.pulse.idle() {
                    if let Some(p) = p.filter(|p| p.ready()) {
                        if !p.save() || p.cursor != 1 {
                            return self.next(Event::LostPopup);
                        }
                        self.go(Stage::SelectYes);
                        self.right_frames = 0;
                        self.neutral = 0;
                        return Output {
                            event: Some(Event::SaveRight),
                            ..self.select(p)
                        };
                    }
                }
            }
            Stage::SelectYes => {
                let Some(p) = p.filter(|p| p.save() && p.ready()) else {
                    return self.next(Event::LostPopup);
                };
                if f.save_status != Some(1) {
                    return self.next(Event::UnexpectedStatus);
                }
                if self.neutral >= 10 && p.cursor == 2 {
                    return self.press(A, Stage::Confirm, Event::SaveConfirm);
                }
                return self.select(p);
            }
            Stage::Confirm => {
                if f.save_status == Some(2) {
                    self.go(Stage::Saving);
                    return Output {
                        event: Some(Event::Saving),
                        ..self.wait()
                    };
                }
                if f.save_status.is_some_and(|s| s != 1) {
                    return self.next(Event::UnexpectedStatus);
                }
                if p.is_some_and(|p| p.record != SAVE_RECORD || p.result == 1 || p.phase == 5) {
                    return self.next(Event::WrongAnswer);
                }
                // Stop holds as soon as the dialog accepts or starts closing.
                if p.is_none_or(|p| !p.ready()) {
                    return Output {
                        input: self.pulse.cancel(),
                        port: self.port,
                        event: None,
                    };
                }
            }
            Stage::Saving => {
                if f.save_status.is_some_and(|s| s != 2 && s != 3) {
                    return self.next(Event::UnexpectedStatus);
                }
                if f.save_status == Some(3)
                    && self.pulse.idle()
                    && p.is_some_and(|p| p.record == SUCCESS_RECORD && p.ready())
                {
                    return self.press(A, Stage::Close, Event::SaveClose);
                }
            }
            Stage::Close => {
                if f.save_status == Some(5) {
                    return self.next(Event::Saved);
                }
                if f.save_status.is_some_and(|s| s != 3) {
                    return self.next(Event::UnexpectedStatus);
                }
                if p.is_none_or(|p| p.record != SUCCESS_RECORD || !p.ready()) {
                    return Output {
                        input: self.pulse.cancel(),
                        port: self.port,
                        event: None,
                    };
                }
            }
            Stage::Rematch => {
                if f.rematch_epoch == self.epoch {
                    self.rematch_seen = true;
                    if let Some(p) = p {
                        if !p.ordinary() || p.expired() {
                            return self.end(Event::LostPopup);
                        }
                        if p.ready() && p.age >= REMATCH_SETTLE && p.cursor == 2 {
                            return self.press(A, Stage::Answer, Event::RematchA);
                        }
                    }
                } else if self.rematch_seen {
                    return self.end(Event::LostPopup);
                }
            }
            Stage::Answer => {
                let Some(p) = p.filter(|p| p.ordinary() && f.rematch_epoch == self.epoch) else {
                    return self.end(Event::LostPopup);
                };
                // Accept only the normal native phase-3 decision, never a
                // retained result, forced answer, or late closing observation.
                if p.phase == 3 && p.pending == 6 && p.decided {
                    return self.end(if p.result == 2 {
                        Event::RematchYes
                    } else {
                        Event::WrongAnswer
                    });
                }
                if p.expired() {
                    return self.end(Event::LostPopup);
                }
                if !p.ready() {
                    return Output {
                        input: self.pulse.cancel(),
                        port: self.port,
                        event: None,
                    };
                }
            }
            _ => {}
        }
        self.wait()
    }
    fn select(&mut self, p: Popup) -> Output {
        let input = if p.cursor == 2 || self.right_frames >= 11 {
            self.neutral = self.neutral.saturating_add(1);
            Input::default()
        } else {
            let first = self.right_frames == 0;
            self.right_frames += 1;
            Input {
                held: RIGHT,
                pressed: if first { RIGHT } else { 0 },
                right: true,
                ..Input::default()
            }
        };
        Output {
            input,
            port: self.port,
            event: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            auto_save_replay: true,
            auto_rematch: true,
            ..Settings::default()
        }
    }
    fn rematch_only() -> Settings {
        Settings {
            auto_save_replay: false,
            ..settings()
        }
    }
    /// Online results: the local fighter's UI port is mapped from the start.
    fn online() -> Frame {
        Frame {
            scene: Scene::Online,
            port: Some(0),
            ..frame()
        }
    }
    /// The save chain after the Y press, dialog to `Saved`, on frame `f`.
    fn save_chain(r: &mut Results, f: &mut Frame) -> Output {
        f.save_status = Some(1);
        f.popup = Some(popup(SAVE_RECORD));
        until(r, *f, Event::SaveRight, 30);
        f.popup.as_mut().unwrap().cursor = 2;
        until(r, *f, Event::SaveConfirm, 15);
        f.save_status = Some(2);
        f.popup = None;
        assert_eq!(r.step(*f, settings()).event, Some(Event::Saving));
        f.save_status = Some(3);
        f.popup = Some(popup(SUCCESS_RECORD));
        until(r, *f, Event::SaveClose, 30);
        f.save_status = Some(5);
        r.step(*f, settings())
    }
    fn frame() -> Frame {
        Frame {
            epoch: 16,
            scene: Scene::Offline,
            port: Some(3),
            cards_seen: true,
            cards_ready: true,
            save_status: Some(0),
            popup: None,
            rematch_epoch: 0,
        }
    }
    /// Offline winner splash: no cards sample yet, hence no port mapping.
    fn splash() -> Frame {
        Frame {
            port: None,
            cards_seen: false,
            cards_ready: false,
            save_status: None,
            ..frame()
        }
    }
    /// Online winner splash (Eden s02): no cards sample, the local entry on UI
    /// port 1 here, the pop-up wrapper idle (phase 0), save status 0.
    fn online_splash() -> Frame {
        Frame {
            scene: Scene::Online,
            port: Some(1),
            cards_seen: false,
            cards_ready: false,
            save_status: Some(0),
            popup: Some(Popup::default()),
            ..frame()
        }
    }
    fn popup(record: u64) -> Popup {
        Popup {
            record,
            phase: 3,
            pending: 3,
            age: 36,
            default_button: if record == REMATCH_RECORD { 2 } else { 1 },
            input_mode: 2,
            cursor: if record == REMATCH_RECORD { 2 } else { 1 },
            ..Popup::default()
        }
    }
    fn until(r: &mut Results, f: Frame, event: Event, limit: u16) -> Output {
        for _ in 0..limit {
            let o = r.step(f, settings());
            // Past the cards sample every input rides the bound port.
            assert!(!matches!(o.event, Some(Event::SplashA(_))));
            if o.input != Input::default() {
                assert_eq!(o.port, f.port, "{:?}", o.event);
            }
            if o.event == Some(event) {
                return o;
            }
            assert!(
                !matches!(
                    o.event,
                    Some(Event::Timeout | Event::LostPopup | Event::UnexpectedStatus)
                ),
                "unexpected {:?} at {:?}",
                o.event,
                r.stage()
            );
        }
        panic!("missing {event:?} at {:?}", r.stage());
    }
    fn begin() -> (Results, Frame) {
        let mut r = Results::new();
        let f = frame();
        assert_eq!(r.step(f, settings()).event, Some(Event::Begin));
        (r, f)
    }
    fn at_select() -> (Results, Frame) {
        let (mut r, mut f) = begin();
        assert_eq!(r.step(f, settings()).event, Some(Event::SaveY));
        f.save_status = Some(1);
        f.popup = Some(popup(SAVE_RECORD));
        until(&mut r, f, Event::SaveRight, 30);
        (r, f)
    }
    fn at_confirm() -> (Results, Frame) {
        let (mut r, mut f) = at_select();
        f.popup.as_mut().unwrap().cursor = 2;
        until(&mut r, f, Event::SaveConfirm, 15);
        (r, f)
    }
    #[test]
    fn complete_save_requires_the_entire_native_status_chain_once() {
        let (mut r, mut f) = at_confirm();
        f.save_status = Some(2);
        f.popup = None;
        assert_eq!(r.step(f, settings()).event, Some(Event::Saving));
        f.save_status = Some(3);
        f.popup = Some(popup(SUCCESS_RECORD));
        until(&mut r, f, Event::SaveClose, 30);
        f.save_status = Some(5);
        assert_eq!(r.step(f, settings()).event, Some(Event::Saved));
        f.save_status = Some(0);
        f.popup = None;
        for _ in 0..2000 {
            assert_eq!(r.step(f, settings()), Output::default());
        }
    }
    #[test]
    fn y_is_one_edge_five_holds_one_release_then_neutral() {
        let (mut r, f) = begin();
        for n in 0..25 {
            let o = r.step(f, settings());
            assert_eq!(o.input.pressed, if n == 0 { Y } else { 0 });
            assert_eq!(o.input.held, if n < 6 { Y } else { 0 });
            assert_eq!(o.input.released, if n == 6 { Y } else { 0 });
        }
    }
    #[test]
    fn direction_needs_a_live_yes_cursor_and_ten_neutral_frames() {
        let (mut r, mut f) = at_select();
        for _ in 0..20 {
            assert_eq!(r.step(f, settings()).input.pressed & A, 0);
        }
        assert_eq!(r.right_frames, 11);
        // No cursor acknowledgement: never send A; eventually terminal.
        let mut sent = false;
        for _ in 0..DIALOG_TIMEOUT {
            sent |= r.step(f, settings()).input.pressed & A != 0;
        }
        assert!(!sent);
        f.popup.as_mut().unwrap().cursor = 2;
        assert_eq!(r.step(f, settings()), Output::default());

        let (mut r, mut f) = at_select();
        f.popup.as_mut().unwrap().cursor = 2;
        for _ in 0..10 {
            assert_eq!(r.step(f, settings()).input.pressed, 0);
        }
        assert_eq!(r.step(f, settings()).input.pressed, A);
    }
    #[test]
    fn each_stage_deadline_wins_over_a_late_signal_and_never_retries() {
        for stage in [
            Stage::Cards,
            Stage::SaveDialog,
            Stage::SelectYes,
            Stage::Confirm,
            Stage::Saving,
            Stage::Close,
            Stage::Rematch,
            Stage::Answer,
        ] {
            let (mut r, mut f) = begin();
            r.stage = stage;
            r.age = match stage {
                Stage::Cards => CARDS_TIMEOUT,
                Stage::Saving => SAVE_TIMEOUT,
                Stage::Close => CLOSE_TIMEOUT,
                Stage::Rematch => REMATCH_WAIT,
                Stage::Answer => ANSWER_TIMEOUT,
                _ => DIALOG_TIMEOUT,
            } - 1;
            r.epoch_age = r.age;
            if matches!(stage, Stage::Rematch | Stage::Answer) {
                r.scene = Scene::Online;
                r.save = false;
                f.scene = Scene::Online;
                f.rematch_epoch = f.epoch;
                f.popup = Some(popup(REMATCH_RECORD));
            }
            let o = r.step(f, settings());
            assert_eq!(o.event, Some(Event::Timeout), "{stage:?}");
            assert_eq!(o.input.pressed, 0);
            for _ in 0..10 {
                assert_eq!(r.step(f, settings()), Output::default());
            }
        }
    }
    #[test]
    fn missing_cards_popup_or_status_times_out_without_a_blind_next_press() {
        for missing in 0..3 {
            let (mut r, mut f) = begin();
            if missing == 0 {
                f.cards_ready = false;
            }
            if missing == 1 {
                f.save_status = None;
            }
            let mut presses = 0;
            let mut timed_out = false;
            for _ in 0..(CARDS_TIMEOUT + DIALOG_TIMEOUT) {
                let o = r.step(f, settings());
                if o.input.pressed != 0 {
                    presses += 1;
                    assert_eq!(o.input.pressed, Y);
                }
                timed_out |= o.event == Some(Event::Timeout);
            }
            assert!(timed_out);
            assert_eq!(presses, if missing == 2 { 1 } else { 0 });
        }
    }
    #[test]
    fn no_old_or_skipped_status_can_report_a_save() {
        for status in [2, 3, 4, 5] {
            let (mut r, mut f) = begin();
            r.step(f, settings());
            f.save_status = Some(status);
            assert!(matches!(
                r.step(f, settings()).event,
                Some(Event::FailedStatus | Event::UnexpectedStatus)
            ));
            f.save_status = Some(5);
            assert_eq!(r.step(f, settings()), Output::default());
        }
        let (mut r, mut f) = at_confirm();
        f.save_status = Some(3);
        f.popup = Some(popup(SUCCESS_RECORD));
        assert_eq!(r.step(f, settings()).event, Some(Event::UnexpectedStatus));
    }
    #[test]
    fn rematch_waits_through_a_long_online_splash() {
        // Eden r01: the pop-up came about 15 s after epoch begin, past the
        // old 480-frame deadline; it must still be answered at frame 1300.
        let (mut r, mut f) = begin();
        r.scene = Scene::Online;
        r.save = false;
        r.stage = Stage::Rematch;
        f.scene = Scene::Online;
        for _ in 0..1300 {
            let o = r.step(f, settings());
            assert_eq!(o.event, None);
            assert_eq!(o.input.pressed, 0);
        }
        f.rematch_epoch = f.epoch;
        f.popup = Some(Popup {
            age: REMATCH_SETTLE,
            ..popup(REMATCH_RECORD)
        });
        let o = r.step(f, settings());
        assert_eq!((o.input.pressed, o.event), (A, Some(Event::RematchA)));
    }

    #[test]
    fn online_rematch_alone_gets_one_a_at_ready_age_36() {
        let mut r = Results::new();
        let mut f = online();
        assert_eq!(r.step(f, rematch_only()).event, Some(Event::Begin));
        assert_eq!(r.stage(), Stage::Rematch);
        f.rematch_epoch = f.epoch;
        f.popup = Some(Popup {
            age: 35,
            ..popup(REMATCH_RECORD)
        });
        assert_eq!(r.step(f, rematch_only()).input, Input::default());
        f.popup.as_mut().unwrap().age = 36;
        let o = r.step(f, rematch_only());
        assert_eq!(o.event, Some(Event::RematchA));
        assert_eq!(o.input.pressed, A);
        for _ in 0..25 {
            assert_eq!(r.step(f, rematch_only()).input.pressed, 0);
        }
        let p = f.popup.as_mut().unwrap();
        p.pending = 6;
        p.result = 2;
        p.decided = true;
        assert_eq!(r.step(f, rematch_only()).event, Some(Event::RematchYes));
        assert_eq!(r.step(f, rematch_only()), Output::default());
    }
    #[test]
    fn online_saves_first_then_answers_the_rematch() {
        let mut r = Results::new();
        let mut f = online();
        assert_eq!(r.step(f, settings()).event, Some(Event::Begin));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed, o.port), (Some(Event::SaveY), Y, Some(0)));
        assert_eq!(save_chain(&mut r, &mut f).event, Some(Event::Saved));
        assert_eq!(r.stage(), Stage::Rematch);
        // The success pop-up lingers; nothing is pressed until the rematch
        // record is parsed in this epoch.
        f.save_status = Some(0);
        for _ in 0..100 {
            let o = r.step(f, settings());
            assert_eq!((o.event, o.input), (None, Input::default()));
        }
        f.rematch_epoch = f.epoch;
        f.popup = Some(popup(REMATCH_RECORD));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed, o.port), (Some(Event::RematchA), A, Some(0)));
        for _ in 0..25 {
            assert_eq!(r.step(f, settings()).input.pressed, 0);
        }
        let p = f.popup.as_mut().unwrap();
        p.pending = 6;
        p.result = 2;
        p.decided = true;
        assert_eq!(r.step(f, settings()).event, Some(Event::RematchYes));
        assert_eq!(r.step(f, settings()), Output::default());
    }
    #[test]
    fn online_save_alone_ends_after_saved() {
        let s = Settings {
            auto_rematch: false,
            ..settings()
        };
        let mut r = Results::new();
        let mut f = online();
        assert_eq!(r.step(f, s).event, Some(Event::Begin));
        assert_eq!(r.step(f, s).event, Some(Event::SaveY));
        assert_eq!(save_chain(&mut r, &mut f).event, Some(Event::Saved));
        assert_eq!(r.stage(), Stage::End);
        f.rematch_epoch = f.epoch;
        f.popup = Some(popup(REMATCH_RECORD));
        for _ in 0..200 {
            assert_eq!(r.step(f, s), Output::default());
        }
    }
    #[test]
    fn online_save_timeout_or_lost_dialog_still_waits_for_the_rematch() {
        // Cards never ready: the save gives up at its deadline, the rematch
        // is still answered, and the rematch deadline counts from epoch begin.
        let mut r = Results::new();
        let mut f = Frame {
            cards_ready: false,
            ..online()
        };
        r.step(f, settings());
        let mut timeout = None;
        for n in 1..=CARDS_TIMEOUT {
            let o = r.step(f, settings());
            assert_eq!(o.input.pressed, 0);
            if o.event == Some(Event::Timeout) {
                timeout = Some(n);
                break;
            }
        }
        assert_eq!(timeout, Some(CARDS_TIMEOUT));
        assert_eq!(r.stage(), Stage::Rematch);
        let mut n = 0u16;
        loop {
            n += 1;
            let o = r.step(f, settings());
            assert_eq!(o.input, Input::default());
            if o.event == Some(Event::Timeout) {
                break;
            }
            assert!(n < 3000);
        }
        assert_eq!(CARDS_TIMEOUT + n, REMATCH_WAIT);
        assert_eq!(r.stage(), Stage::End);
        // Answered in time when the pop-up comes.
        let mut r = Results::new();
        r.step(f, settings());
        for _ in 0..CARDS_TIMEOUT {
            r.step(f, settings());
        }
        assert_eq!(r.stage(), Stage::Rematch);
        f.rematch_epoch = f.epoch;
        f.popup = Some(popup(REMATCH_RECORD));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed), (Some(Event::RematchA), A));
        // A dialog replaced by the rematch pop-up mid-save gives way too.
        let mut r = Results::new();
        let mut f = online();
        r.step(f, settings());
        assert_eq!(r.step(f, settings()).event, Some(Event::SaveY));
        f.save_status = Some(1);
        f.popup = Some(popup(SAVE_RECORD));
        until(&mut r, f, Event::SaveRight, 30);
        f.popup = Some(popup(REMATCH_RECORD));
        f.rematch_epoch = f.epoch;
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.held), (Some(Event::LostPopup), 0));
        assert_eq!(r.stage(), Stage::Rematch);
        f.save_status = Some(0);
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed), (Some(Event::RematchA), A));
    }
    #[test]
    fn online_rematch_popup_before_the_save_gives_way_to_the_rematch() {
        let mut r = Results::new();
        let mut f = Frame {
            cards_ready: false,
            ..online()
        };
        r.step(f, settings());
        for _ in 0..300 {
            let o = r.step(f, settings());
            assert_eq!((o.event, o.input), (None, Input::default()));
        }
        f.rematch_epoch = f.epoch;
        f.popup = Some(popup(REMATCH_RECORD));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed), (Some(Event::SaveLate), 0));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed), (Some(Event::RematchA), A));
        // Without Auto rematch the late save just ends.
        let s = Settings {
            auto_rematch: false,
            ..settings()
        };
        let mut r = Results::new();
        r.step(f, s);
        assert_eq!(r.step(f, s).event, Some(Event::SaveLate));
        assert_eq!(r.stage(), Stage::End);
    }
    #[test]
    fn online_save_row_turned_off_mid_epoch_moves_on_to_the_rematch() {
        let mut r = Results::new();
        let f = online();
        r.step(f, settings());
        assert_eq!(r.step(f, settings()).event, Some(Event::SaveY));
        let o = r.step(f, rematch_only());
        assert_eq!((o.event, o.input.released), (Some(Event::Cancelled), Y));
        assert_eq!(r.stage(), Stage::Rematch);
        // Both rows off: the epoch ends.
        assert_eq!(r.step(f, Settings::default()).event, Some(Event::Cancelled));
        assert_eq!(r.stage(), Stage::End);
    }
    #[test]
    fn rematch_does_not_accept_an_unqualified_or_changed_popup() {
        for variant in 0..8 {
            let mut r = Results::new();
            let mut f = online();
            r.step(f, rematch_only());
            f.rematch_epoch = f.epoch;
            let mut p = popup(REMATCH_RECORD);
            match variant {
                0 => p.record = SAVE_RECORD,
                1 => p.default_button = 1,
                2 => p.cancel = 1,
                3 => p.input_mode = 1,
                4 => p.phase = 5,
                5 => p.result = 2,
                6 => p.decided = true,
                _ => p.pending = 6,
            }
            f.popup = Some(p);
            let o = r.step(f, rematch_only());
            assert_eq!(o.input.pressed, 0);
            assert_eq!(o.event, Some(Event::LostPopup));
        }
    }
    #[test]
    fn forced_late_no_and_unknown_answers_never_count_as_rematch_yes() {
        for (phase, result) in [(5, 2), (6, 2), (0, 2), (3, 1), (3, 0), (3, 3)] {
            let mut r = Results::new();
            let mut f = online();
            r.step(f, rematch_only());
            f.rematch_epoch = f.epoch;
            f.popup = Some(popup(REMATCH_RECORD));
            r.step(f, rematch_only());
            let p = f.popup.as_mut().unwrap();
            p.phase = phase;
            p.pending = 6;
            p.decided = true;
            p.result = result;
            let o = r.step(f, rematch_only());
            assert_ne!(o.event, Some(Event::RematchYes));
            assert_eq!(o.input.pressed, 0);
            assert_eq!(r.stage(), Stage::End);
        }
    }
    #[test]
    fn disable_scene_end_port_change_and_replaced_dialog_cancel_inputs() {
        for variant in 0..4 {
            let (mut r, mut f) = at_select();
            let mut s = settings();
            match variant {
                0 => s.auto_save_replay = false,
                1 => f.epoch = 0,
                2 => f.port = Some(7),
                _ => f.popup = Some(popup(REMATCH_RECORD)),
            }
            let o = r.step(f, s);
            assert_eq!(o.input.held, 0);
            assert_eq!(o.input.pressed, 0);
        }
    }
    #[test]
    fn reused_results_address_after_absence_starts_a_new_epoch() {
        let (mut r, mut f) = begin();
        assert_eq!(r.step(f, settings()).input.pressed, Y);
        f.epoch = 0;
        let o = r.step(f, settings());
        assert_eq!(o.input.released, Y);
        assert_eq!(o.port, Some(3));
        f.epoch = 16;
        r.step(f, settings());
        assert_eq!(r.step(f, settings()).input.pressed, Y);
    }
    #[test]
    fn port_binding_is_bounded_and_native_other_records_are_untouched() {
        let original = Record {
            held: 0x12,
            pressed: 2,
            left: [0.25, 0.5, 0.75, 1.0],
            right: [1.0; 4],
            ..Record::default()
        };
        let mut records = [original; 10];
        Input {
            held: RIGHT,
            pressed: RIGHT,
            right: true,
            ..Input::default()
        }
        .apply(&mut records, 3);
        for (i, r) in records.iter().enumerate() {
            if i != 3 {
                assert_eq!(*r, original);
            }
        }
        assert_eq!(records[3].left, [1.0, 0.0, 0.75, 0.0]);
        assert_eq!(records[3].right, original.right);
        let before = records;
        Input {
            held: A,
            ..Input::default()
        }
        .apply(&mut records, 10);
        assert_eq!(records, before);
        let mut r = Results::new();
        let f = Frame {
            port: Some(10),
            ..frame()
        };
        r.step(f, settings());
        for _ in 0..10 {
            assert_eq!(r.step(f, settings()).input.pressed, 0);
        }
    }
    #[test]
    fn settings_off_and_unknown_playback_training_modes_never_press() {
        // (melee, mode): nothing, replay playback, an unknown family, Training.
        for (melee, mode) in [(0, 0), (0, 0x05010100), (0x20, 0x04040000), (0x0d, 0x02010000)] {
            let mut r = Results::new();
            let f = Frame {
                scene: scene(true, melee, mode),
                ..frame()
            };
            for _ in 0..10 {
                assert_eq!(r.step(f, settings()).input, Input::default());
            }
        }
        assert_eq!(scene(false, 0, 0x02010000), Scene::Unsupported);
        // Local Smash results (Eden c14) and the earlier unproven family.
        assert_eq!(scene(true, 0, 0x02010000), Scene::Offline);
        assert_eq!(scene(true, 0, 0x01010000), Scene::Unsupported);
        assert_eq!(scene(true, 0x0e, 0x07010101), Scene::Online);
        assert_eq!(scene(true, 0, 0x07010000), Scene::Online);
        let mut r = Results::new();
        for _ in 0..10 {
            assert_eq!(r.step(frame(), Settings::default()).input, Input::default());
        }
    }
    #[test]
    fn splash_a_waits_the_settle_delay_then_is_one_edge_five_holds_one_release_on_port_0() {
        let mut r = Results::new();
        let f = splash();
        assert_eq!(r.step(f, settings()).event, Some(Event::Begin));
        for n in 1..SPLASH_SETTLE {
            assert_eq!(r.step(f, settings()), Output::default(), "frame {n}");
        }
        let o = r.step(f, settings());
        assert_eq!(o.event, Some(Event::SplashA(1)));
        assert_eq!(o.port, Some(SPLASH_PORT));
        assert_eq!(o.input.pressed, A);
        assert_eq!(o.input.held, A);
        for n in 1..25 {
            let o = r.step(f, settings());
            assert_eq!(o.event, None);
            assert_eq!(o.input.pressed, 0);
            assert_eq!(o.input.held, if n < 6 { A } else { 0 });
            assert_eq!(o.input.released, if n == 6 { A } else { 0 });
            if o.input != Input::default() {
                assert_eq!(o.port, Some(SPLASH_PORT));
            }
        }
    }
    #[test]
    fn splash_a_repeats_every_gap_at_most_splash_max_times_and_the_deadline_restarts_at_the_last() {
        let mut r = Results::new();
        let f = splash();
        r.step(f, settings());
        let mut presses = Vec::new();
        let mut timeout = None;
        for n in 1..=4000u32 {
            let o = r.step(f, settings());
            if o.input != Input::default() {
                assert_eq!(o.port, Some(SPLASH_PORT));
            }
            if o.input.pressed != 0 {
                assert_eq!(o.input.pressed, A);
                presses.push(n);
                assert_eq!(o.event, Some(Event::SplashA(presses.len() as u8)));
            }
            if o.event == Some(Event::Timeout) {
                timeout = Some(n);
                break;
            }
        }
        let first = u32::from(SPLASH_SETTLE);
        let gap = u32::from(SPLASH_GAP);
        let expected: Vec<u32> = (0..u32::from(SPLASH_MAX)).map(|k| first + k * gap).collect();
        assert_eq!(presses, expected);
        assert_eq!(presses.len(), usize::from(SPLASH_MAX));
        let last = *presses.last().unwrap();
        assert_eq!(timeout, Some(last + u32::from(CARDS_TIMEOUT)));
        // Terminal: nothing more in this epoch.
        for _ in 0..500 {
            assert_eq!(r.step(f, settings()), Output::default());
        }
    }
    #[test]
    fn no_splash_a_once_any_cards_sample_exists_even_an_unready_or_portless_one() {
        // (A presses before the sample, the sample's port).
        for (before, port) in [(0, Some(3)), (1, Some(3)), (2, Some(3)), (1, None)] {
            let mut r = Results::new();
            let mut f = splash();
            r.step(f, settings());
            let mut presses = 0;
            while presses < before {
                presses += u8::from(r.step(f, settings()).input.pressed == A);
            }
            f.cards_seen = true;
            f.port = port;
            let mut timed_out = false;
            for _ in 0..(CARDS_TIMEOUT + 4 * SPLASH_GAP) {
                let o = r.step(f, settings());
                assert_eq!(o.input.pressed, 0, "before={before} port={port:?}");
                timed_out |= o.event == Some(Event::Timeout);
            }
            assert!(timed_out);
            assert_eq!(r.splash, before);
        }
    }
    #[test]
    fn a_splash_a_in_flight_is_released_on_its_own_port_when_cards_arrive() {
        let mut r = Results::new();
        let mut f = splash();
        r.step(f, settings());
        while r.step(f, settings()).input.pressed != A {}
        for _ in 0..2 {
            let o = r.step(f, settings());
            assert_eq!((o.input.held, o.port), (A, Some(SPLASH_PORT)));
        }
        f.cards_seen = true;
        f.port = Some(3);
        let o = r.step(f, settings());
        assert_eq!(o.input.held, 0);
        assert_eq!(o.input.pressed, 0);
        assert_eq!(o.input.released, A);
        assert_eq!(o.port, Some(SPLASH_PORT));
        // The cards port was bound on that frame; nothing is held on any port.
        f.cards_ready = true;
        f.save_status = Some(0);
        let o = r.step(f, settings());
        assert_eq!(o.event, Some(Event::SaveY));
        assert_eq!(o.port, Some(3));
        // A new epoch (or a lost epoch) releases a live splash A on port 0.
        let mut r = Results::new();
        let mut f = splash();
        r.step(f, settings());
        while r.step(f, settings()).input.pressed != A {}
        r.step(f, settings());
        f.epoch = 0;
        let o = r.step(f, settings());
        assert_eq!((o.input.released, o.port), (A, Some(SPLASH_PORT)));
    }
    #[test]
    fn splash_a_never_presses_in_training_playback_unsupported_or_save_off() {
        let steps = SPLASH_SETTLE + 4 * SPLASH_GAP;
        // (melee, mode): Training, replay playback, unknown family, nothing.
        for (melee, mode) in [(0x0d, 0x0201_0000), (0, 0x0501_0100), (0x20, 0x0404_0000), (0, 0)] {
            let mut r = Results::new();
            let f = Frame {
                scene: scene(true, melee, mode),
                ..splash()
            };
            assert_ne!(f.scene, Scene::Offline);
            for _ in 0..steps {
                assert_eq!(r.step(f, settings()).input, Input::default());
            }
        }
        // Online with Auto save off (the rematch alone) never presses the splash.
        let mut r = Results::new();
        let f = online_splash();
        for _ in 0..steps {
            let o = r.step(f, rematch_only());
            assert_eq!(o.input, Input::default());
            assert!(!matches!(o.event, Some(Event::SplashA(_))));
        }
        // auto_save_replay off from the start, and turned off mid-epoch.
        let off = Settings {
            auto_save_replay: false,
            ..settings()
        };
        let mut r = Results::new();
        for _ in 0..steps {
            assert_eq!(r.step(splash(), off).input, Input::default());
        }
        let mut r = Results::new();
        for _ in 0..SPLASH_SETTLE - 10 {
            r.step(splash(), settings());
        }
        assert_eq!(r.step(splash(), off).event, Some(Event::Cancelled));
        for _ in 0..steps {
            assert_eq!(r.step(splash(), settings()), Output::default());
        }
    }
    #[test]
    fn full_save_sequence_still_completes_when_cards_arrive_after_the_second_a() {
        let mut r = Results::new();
        let mut f = splash();
        r.step(f, settings());
        let mut presses = 0;
        for _ in 0..(SPLASH_SETTLE + SPLASH_GAP + 20) {
            let o = r.step(f, settings());
            if o.input.pressed == A {
                presses += 1;
                assert_eq!(o.port, Some(SPLASH_PORT));
            }
        }
        assert_eq!(presses, 2);
        // Cards appear: ready on the very first sample, with P1's real port.
        f.cards_seen = true;
        f.cards_ready = true;
        f.port = Some(3);
        f.save_status = Some(0);
        let o = r.step(f, settings());
        assert_eq!(o.event, Some(Event::SaveY));
        assert_eq!((o.input.pressed, o.port), (Y, Some(3)));
        f.save_status = Some(1);
        f.popup = Some(popup(SAVE_RECORD));
        until(&mut r, f, Event::SaveRight, 30);
        f.popup.as_mut().unwrap().cursor = 2;
        until(&mut r, f, Event::SaveConfirm, 15);
        f.save_status = Some(2);
        f.popup = None;
        assert_eq!(r.step(f, settings()).event, Some(Event::Saving));
        f.save_status = Some(3);
        f.popup = Some(popup(SUCCESS_RECORD));
        until(&mut r, f, Event::SaveClose, 30);
        f.save_status = Some(5);
        assert_eq!(r.step(f, settings()).event, Some(Event::Saved));
        assert_eq!(r.splash, 2);
        f.save_status = Some(0);
        f.popup = None;
        for _ in 0..(3 * SPLASH_GAP + 500) {
            assert_eq!(r.step(f, settings()), Output::default());
        }
    }
    #[test]
    fn online_splash_a_rides_the_mapped_port_until_the_cards_then_saves_and_answers() {
        let mut r = Results::new();
        let mut f = online_splash();
        assert_eq!(r.step(f, settings()).event, Some(Event::Begin));
        let mut presses = Vec::new();
        for n in 1..=(SPLASH_SETTLE + SPLASH_GAP + 10) {
            let o = r.step(f, settings());
            if o.input != Input::default() {
                assert_eq!(o.port, Some(1));
            }
            if o.input.pressed != 0 {
                assert_eq!(o.input.pressed, A);
                presses.push(n);
                assert_eq!(o.event, Some(Event::SplashA(presses.len() as u8)));
            }
        }
        assert_eq!(presses, [SPLASH_SETTLE, SPLASH_SETTLE + SPLASH_GAP]);
        // The cards come up (not ready yet): no further A, no Y.
        f.cards_seen = true;
        for _ in 0..(2 * SPLASH_GAP) {
            assert_eq!(r.step(f, settings()).input.pressed, 0);
        }
        // Ready: Y on the mapped port, then the whole save, then the rematch.
        f.cards_ready = true;
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed, o.port), (Some(Event::SaveY), Y, Some(1)));
        assert_eq!(save_chain(&mut r, &mut f).event, Some(Event::Saved));
        assert_eq!(r.stage(), Stage::Rematch);
        f.save_status = Some(0);
        f.rematch_epoch = f.epoch;
        f.popup = Some(popup(REMATCH_RECORD));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed, o.port), (Some(Event::RematchA), A, Some(1)));
        assert_eq!(r.splash, 2);
    }
    #[test]
    fn online_splash_a_never_reaches_a_popup_and_a_parsed_rematch_ends_the_presses() {
        // A pop-up opening or shown before the first press: no splash A at all.
        for busy in [
            Popup {
                opening: true,
                ..Popup::default()
            },
            popup(REMATCH_RECORD),
        ] {
            let mut r = Results::new();
            let f = Frame {
                popup: Some(busy),
                ..online_splash()
            };
            r.step(f, settings());
            for _ in 0..(SPLASH_SETTLE + 4 * SPLASH_GAP) {
                assert_eq!(r.step(f, settings()).input, Input::default());
            }
            assert_eq!(r.splash, 0);
        }
        // A pop-up starting while the A is held: released at once on its port.
        let mut r = Results::new();
        let mut f = online_splash();
        r.step(f, settings());
        while r.step(f, settings()).input.pressed != A {}
        f.popup = Some(Popup {
            phase: 1,
            ..Popup::default()
        });
        let o = r.step(f, settings());
        assert_eq!((o.input.held, o.input.pressed, o.input.released), (0, 0, A));
        assert_eq!(o.port, Some(1));
        for _ in 0..(2 * SPLASH_GAP) {
            assert_eq!(r.step(f, settings()).input, Input::default());
        }
        // The "Rematch?" record parsed: the save is given up, the rematch answered.
        f.rematch_epoch = f.epoch;
        f.popup = Some(popup(REMATCH_RECORD));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed), (Some(Event::SaveLate), 0));
        let o = r.step(f, settings());
        assert_eq!((o.event, o.input.pressed, o.port), (Some(Event::RematchA), A, Some(1)));
        assert_eq!(r.splash, 1);
        // Without Auto rematch the same pop-up only ends the epoch: nothing pressed.
        let s = Settings {
            auto_rematch: false,
            ..settings()
        };
        let mut r = Results::new();
        let mut f = online_splash();
        r.step(f, s);
        for _ in 0..SPLASH_SETTLE {
            r.step(f, s);
        }
        f.rematch_epoch = f.epoch;
        f.popup = Some(popup(REMATCH_RECORD));
        let mut events = Vec::new();
        for _ in 0..(4 * SPLASH_GAP) {
            let o = r.step(f, s);
            assert_eq!(o.input.pressed, 0);
            events.extend(o.event);
        }
        assert_eq!(events, [Event::SaveLate]);
        assert_eq!(r.stage(), Stage::End);
    }
}
