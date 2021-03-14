use std::{borrow::BorrowMut, fs::File};

use rodio::{self, Source};

#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Left,
    Center,
    Right,
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum Sounds {
    BossInjured,
    BossDied,
    Bump,
    DrawerOpen,
    EnemyDeath,
    FloorRaise,
    FireballShoot,
    FireballHitsWall,
    FirebrandInjury,
    FirebrandDeath,
    PowerUp,
}

impl Sounds {
    fn file(&self) -> File {
        use Sounds::*;
        std::fs::File::open(match self {
            BossInjured => "res/audio/boss_injury.wav",
            BossDied => "res/audio/boss_death.wav",
            Bump => "res/audio/bump.wav",
            DrawerOpen => "res/audio/drawer_open.wav",
            EnemyDeath => "res/audio/enemy_death.wav",
            FloorRaise => "res/audio/floor_raise.wav",
            FireballShoot => "res/audio/fireball.wav",
            FireballHitsWall => "res/audio/fireball_hit.wav",
            FirebrandInjury => "res/audio/fb_injury.wav",
            FirebrandDeath => "res/audio/fb_death.wav",
            PowerUp => "res/audio/powerup.wav",
        })
        .unwrap()
    }

    fn buffer(&self) -> std::io::BufReader<File> {
        std::io::BufReader::new(self.file())
    }

    fn should_pause_current_track(&self) -> bool {
        use Sounds::*;
        // PowerUp interrupts audio track; all else can play simultaneously
        matches!(self, PowerUp | FirebrandDeath)
    }

    fn volume(&self) -> f32 {
        use Sounds::*;
        match self {
            FireballShoot => 0.5,
            FireballHitsWall => 0.675,
            FirebrandInjury => 0.3,
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum Tracks {
    MainTheme,
    BossFight,
    AreaClear,
    GameOver,
}

impl Tracks {
    fn file(&self) -> File {
        use Tracks::*;
        std::fs::File::open(match self {
            MainTheme => "res/audio/main_theme.ogg",
            BossFight => "res/audio/boss_fight.ogg",
            AreaClear => "res/audio/area_clear.ogg",
            GameOver => "res/audio/game_over.ogg",
        })
        .unwrap()
    }

    fn buffer(&self) -> std::io::BufReader<File> {
        std::io::BufReader::new(self.file())
    }

    fn volume(&self) -> f32 {
        0.5
    }

    fn loops(&self) -> bool {
        use Tracks::*;
        match self {
            MainTheme => true,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

pub struct Audio {
    stream: rodio::OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    current_track: Option<rodio::Sink>,
    current_track_explicitly_paused: bool,
    sinks: Vec<rodio::Sink>,
    interrupting_sinks: Vec<rodio::Sink>,
}

impl Default for Audio {
    fn default() -> Self {
        let (stream, stream_handle) =
            rodio::OutputStream::try_default().expect("Expect to open rodio audio output");
        Audio {
            stream,
            stream_handle,
            current_track: None,
            current_track_explicitly_paused: false,
            sinks: Vec::new(),
            interrupting_sinks: Vec::new(),
        }
    }
}

impl Audio {
    pub fn update(&mut self, _dt: std::time::Duration) {
        // prune sinks
        self.sinks.retain(|s| !s.empty());
        self.interrupting_sinks.retain(|s| !s.empty());

        let pause_current_track =
            self.current_track_explicitly_paused || !self.interrupting_sinks.is_empty();

        if let Some(current_track) = self.current_track.borrow_mut() {
            if pause_current_track && !current_track.is_paused() {
                current_track.pause();
            } else if !pause_current_track && current_track.is_paused() {
                current_track.play();
            }
        }
    }

    pub fn start_track(&mut self, track: Tracks) {
        self.stop_current_track();
        println!("Audio::play_track {:?}", track);
        let sink = rodio::Sink::try_new(&self.stream_handle).unwrap();
        sink.set_volume(track.volume());

        let source = rodio::Decoder::new(track.buffer()).unwrap();
        if track.loops() {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }

        self.current_track = Some(sink);
        self.current_track_explicitly_paused = false;
    }

    pub fn pause_current_track(&mut self) {
        if self.current_track.is_some() {
            self.current_track_explicitly_paused = true;
        }
    }

    pub fn resume_current_track(&mut self) {
        if self.current_track.is_some() {
            self.current_track_explicitly_paused = false;
        }
    }

    pub fn current_track_is_paused(&mut self) -> bool {
        self.current_track_explicitly_paused
    }

    pub fn stop_current_track(&mut self) {
        if let Some(sink) = self.current_track.borrow_mut() {
            println!("Audio::stop_current_track");
            sink.stop();
        }
        self.current_track = None;
    }

    pub fn play_sound(&mut self, sound: Sounds) {
        println!("Audio::play_sound {:?}", sound);
        let sink = self.stream_handle.play_once(sound.buffer()).unwrap();
        sink.set_volume(sound.volume());
        if sound.should_pause_current_track() {
            self.interrupting_sinks.push(sink);
        } else {
            // TODO: Can probably just use sink::detach and lose these single-shot deals
            self.sinks.push(sink);
        }
    }
}
