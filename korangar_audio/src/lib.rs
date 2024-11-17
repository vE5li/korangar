//! This crate exposes an audio engine for the client
#![feature(let_chains)]
#![forbid(missing_docs)]

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::mem::swap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cgmath::{InnerSpace, Matrix3, Point3, Quaternion, Vector3};
use cpal::BufferSize;
use kira::manager::backend::cpal::{CpalBackend, CpalBackendSettings};
use kira::manager::{AudioManager, AudioManagerSettings, Capacities};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::{FromFileError, PlaybackState};
use kira::spatial::emitter::{EmitterDistances, EmitterHandle, EmitterSettings};
use kira::spatial::listener::{ListenerHandle, ListenerSettings};
use kira::spatial::scene::{SpatialSceneHandle, SpatialSceneSettings};
use kira::track::{TrackBuilder, TrackHandle};
use kira::tween::{Easing, Tween, Value};
use kira::{Frame, Volume};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use korangar_util::collision::{KDTree, Sphere};
use korangar_util::container::{Cacheable, GenerationalSlab, SimpleCache, SimpleSlab};
use korangar_util::{create_generational_key, create_simple_key, FileLoader};
use rayon::spawn;

create_generational_key!(SoundEffectKey, "The key for a cached sound effect");
create_simple_key!(AmbientKey, "The key for a ambient sound");

const MAX_QUEUE_TIME_SECONDS: f32 = 1.0;
const MAX_CACHE_COUNT: u32 = 400;
const MAX_CACHE_SIZE: usize = 50 * 104 * 1024; // 50 MiB
const SOUND_EFFECT_BASE_PATH: &str = "data\\wav";
const BACKGROUND_MUSIC_MAPPING_FILE: &str = "data\\mp3NameTable.txt";

struct BackgroundMusicTrack {
    track_name: String,
    handle: StreamingSoundHandle<FromFileError>,
}

struct QueuedSoundEffect {
    /// The key of the sound that should be played.
    sound_effect_key: SoundEffectKey,
    /// The optional key to the ambient sound emitter.
    ambient: Option<AmbientKey>,
    /// The time this playback was queued.
    queued_time: Instant,
}

struct AmbientSoundConfig {
    sound_effect_key: SoundEffectKey,
    bounds: Sphere,
    volume: f32,
    cycle: Option<f32>,
}

struct PlayingAmbient {
    data: StaticSoundData,
    handle: StaticSoundHandle,
    cycle: f32,
    last_start: Instant,
}

#[repr(transparent)]
struct CachedSoundEffect(StaticSoundData);

impl Cacheable for CachedSoundEffect {
    fn size(&self) -> usize {
        self.0.frames.len() * size_of::<Frame>()
    }
}

enum AsyncLoadResult {
    Loaded {
        path: String,
        key: SoundEffectKey,
        sound_effect: Box<StaticSoundData>,
    },
    Error {
        path: String,
        key: SoundEffectKey,
        message: String,
    },
}

/// The audio engine of Korangar. Provides a simple interface to play background
/// music, short sounds (sound effects) and spatial, ambient sound (sounds on
/// the map).
pub struct AudioEngine<F> {
    engine_context: Mutex<EngineContext<F>>,
}

struct EngineContext<F> {
    active_emitters: HashMap<AmbientKey, EmitterHandle>,
    ambient_listener: ListenerHandle,
    ambient_sound: SimpleSlab<AmbientKey, AmbientSoundConfig>,
    ambient_track: TrackHandle,
    async_response_receiver: Receiver<AsyncLoadResult>,
    async_response_sender: Sender<AsyncLoadResult>,
    background_music_track: TrackHandle,
    background_music_track_mapping: HashMap<String, String>,
    cache: SimpleCache<SoundEffectKey, CachedSoundEffect>,
    current_background_music_track: Option<BackgroundMusicTrack>,
    cycling_ambient: HashMap<AmbientKey, PlayingAmbient>,
    game_file_loader: Arc<F>,
    last_listener_update: Instant,
    loading_sound_effect: HashSet<SoundEffectKey>,
    lookup: HashMap<String, SoundEffectKey>,
    manager: AudioManager,
    object_kdtree: KDTree<AmbientKey, Sphere>,
    previous_query_result: Vec<AmbientKey>,
    query_result: Vec<AmbientKey>,
    queued_background_music_track: Option<String>,
    queued_sound_effect: Vec<QueuedSoundEffect>,
    scene: SpatialSceneHandle,
    scratchpad: Vec<AmbientKey>,
    sound_effect_paths: GenerationalSlab<SoundEffectKey, String>,
    sound_effect_track: TrackHandle,
}

impl<F: FileLoader> AudioEngine<F> {
    /// Crates a new audio engine.
    pub fn new(game_file_loader: Arc<F>) -> AudioEngine<F> {
        let mut manager = AudioManager::<CpalBackend>::new(AudioManagerSettings {
            capacities: Capacities::default(),
            main_track_builder: TrackBuilder::default(),
            backend_settings: CpalBackendSettings {
                device: None,
                // At sampling rate of 48 kHz 1200 frames take 25 ms.
                buffer_size: BufferSize::Fixed(1200),
            },
        })
        .expect("Can't initialize audio backend");
        let mut scene = manager
            .add_spatial_scene(SpatialSceneSettings::default())
            .expect("Can't create spatial scene");
        let background_music_track = manager
            .add_sub_track(TrackBuilder::new())
            .expect("Can't create background music track");
        let sound_effect_track = manager.add_sub_track(TrackBuilder::new()).expect("Can't create sound effect track");
        let ambient_track = manager.add_sub_track(TrackBuilder::new()).expect("Can't create ambient track");
        let position = Vector3::new(0.0, 0.0, 0.0);
        let orientation = Quaternion::new(0.0, 0.0, 0.0, 0.0);
        let ambient_listener = scene
            .add_listener(position, orientation, ListenerSettings { track: ambient_track.id() })
            .expect("Can't create ambient listener");
        let loading_sound_effect = HashSet::new();
        let cache = SimpleCache::new(
            NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
            NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
        );
        let (async_response_sender, async_response_receiver) = channel();

        let background_music_track_mapping = parse_background_music_track_mapping(game_file_loader.deref());

        let object_kdtree = KDTree::empty();

        let engine_context = Mutex::new(EngineContext {
            active_emitters: HashMap::default(),
            ambient_listener,
            ambient_sound: SimpleSlab::default(),
            ambient_track,
            async_response_receiver,
            async_response_sender,
            background_music_track,
            background_music_track_mapping,
            cache,
            current_background_music_track: None,
            cycling_ambient: HashMap::default(),
            game_file_loader,
            last_listener_update: Instant::now(),
            loading_sound_effect,
            lookup: HashMap::default(),
            manager,
            object_kdtree,
            previous_query_result: Vec::default(),
            query_result: Vec::default(),
            queued_background_music_track: None,
            queued_sound_effect: Vec::default(),
            scene,
            scratchpad: Vec::default(),
            sound_effect_paths: GenerationalSlab::default(),
            sound_effect_track,
        });

        AudioEngine { engine_context }
    }

    /// This function needs the full file path with the file extension.
    pub fn get_track_for_map(&self, map_file_path: &str) -> Option<String> {
        let context = self.engine_context.lock().unwrap();

        let path = match cfg!(target_os = "windows") {
            true => PathBuf::from(map_file_path),
            false => PathBuf::from(map_file_path.replace('\\', "/")),
        };

        let file_name = path.file_name()?.to_string_lossy();
        context.background_music_track_mapping.get(file_name.as_ref()).cloned()
    }

    /// Registers the given audio file path, queues it's loading and returns a
    /// key. If the audio file path was already registers, it will simply return
    /// its key.
    pub fn load(&self, path: &str) -> SoundEffectKey {
        let mut context = self.engine_context.lock().unwrap();

        if let Some(sound_effect_key) = context.lookup.get(path) {
            return *sound_effect_key;
        }

        let sound_effect_key = context.sound_effect_paths.insert(path.to_string()).expect("Mapping slab is full");
        context.lookup.insert(path.to_string(), sound_effect_key);

        spawn_async_load(
            context.game_file_loader.clone(),
            context.async_response_sender.clone(),
            path.to_string(),
            sound_effect_key,
        );

        sound_effect_key
    }

    /// Unloads und unregisters the registered audio file.
    pub fn unload(&self, sound_effect_key: SoundEffectKey) {
        let mut context = self.engine_context.lock().unwrap();

        if let Some(path) = context.sound_effect_paths.remove(sound_effect_key) {
            let _ = context.lookup.remove(&path);
        }
        context.loading_sound_effect.remove(&sound_effect_key);
        let _ = context.cache.remove(&sound_effect_key);
    }

    /// Sets the global volume.
    pub fn set_main_volume(&self, volume: impl Into<Value<Volume>>) {
        self.engine_context.lock().unwrap().set_main_volume(volume)
    }

    /// Sets the volume of the background music.
    pub fn set_background_music_volume(&self, volume: impl Into<Value<Volume>>) {
        self.engine_context.lock().unwrap().set_background_music_volume(volume)
    }

    /// Sets the volume of sound effect.
    pub fn set_sound_effect_volume(&self, volume: impl Into<Value<Volume>>) {
        self.engine_context.lock().unwrap().set_sound_effect_volume(volume)
    }

    /// Sets the volume of ambient sounds.
    pub fn set_ambient_volume(&self, volume: impl Into<Value<Volume>>) {
        self.engine_context.lock().unwrap().set_ambient_volume(volume)
    }

    /// Plays the background music track. Fades out the currently playing
    /// background music track and then start the new background music
    /// track.
    pub fn play_background_music_track(&self, track_name: Option<&str>) {
        self.engine_context.lock().unwrap().play_background_music_track(track_name)
    }

    /// Plays a sound effect.
    pub fn play_sound_effect(&self, sound_effect_key: SoundEffectKey) {
        self.engine_context.lock().unwrap().play_sound_effect(sound_effect_key)
    }

    /// Sets the listener of the ambient sound. This is normally the camera's
    /// position and orientation. This should update each frame.
    pub fn set_ambient_listener(&self, position: Point3<f32>, view_direction: Vector3<f32>, look_up: Vector3<f32>) {
        self.engine_context
            .lock()
            .unwrap()
            .set_ambient_listener(position, view_direction, look_up)
    }

    /// Ambient sound loops and needs to be removed once the player it outside
    /// the ambient sound range.
    ///
    /// [`prepare_ambient_sound_world()`] must be called once all ambient sound
    /// have been added.
    pub fn add_ambient_sound(
        &self,
        sound_effect_key: SoundEffectKey,
        position: Point3<f32>,
        range: f32,
        volume: f32,
        cycle: Option<f32>,
    ) -> AmbientKey {
        self.engine_context
            .lock()
            .unwrap()
            .add_ambient_sound(sound_effect_key, position, range, volume, cycle)
    }

    /// Removes an ambient sound.
    pub fn remove_ambient_sound(&self, ambient_key: AmbientKey) {
        self.engine_context.lock().unwrap().remove_ambient_sound(ambient_key)
    }

    /// Removes all ambient sound emitters from the spatial scene.
    pub fn clear_ambient_sound(&self) {
        self.engine_context.lock().unwrap().clear_ambient_sound()
    }

    /// Re-creates the spatial world with the ambient sounds.
    pub fn prepare_ambient_sound_world(&self) {
        self.engine_context.lock().unwrap().prepare_ambient_sound_world()
    }

    /// Updates the internal state of the audio engine. Should be called once
    /// each frame.
    pub fn update(&self) {
        self.engine_context.lock().unwrap().update()
    }
}

impl<F: FileLoader> EngineContext<F> {
    fn set_main_volume(&mut self, volume: impl Into<Value<Volume>>) {
        self.manager.main_track().set_volume(volume, Tween {
            duration: Duration::from_millis(500),
            ..Default::default()
        });
    }

    fn set_background_music_volume(&mut self, volume: impl Into<Value<Volume>>) {
        self.background_music_track.set_volume(volume, Tween {
            duration: Duration::from_millis(500),
            ..Default::default()
        });
    }

    fn set_sound_effect_volume(&mut self, volume: impl Into<Value<Volume>>) {
        self.sound_effect_track.set_volume(volume, Tween {
            duration: Duration::from_millis(500),
            ..Default::default()
        });
    }

    fn set_ambient_volume(&mut self, volume: impl Into<Value<Volume>>) {
        self.ambient_track.set_volume(volume, Tween {
            duration: Duration::from_millis(500),
            ..Default::default()
        });
    }

    fn play_background_music_track(&mut self, track_name: Option<&str>) {
        let Some(track_name) = track_name else {
            if let Some(playing) = self.current_background_music_track.as_mut() {
                playing.handle.stop(Tween {
                    duration: Duration::from_secs(1),
                    ..Default::default()
                });
            }

            self.current_background_music_track = None;
            return;
        };

        if let Some(playing) = self.current_background_music_track.as_mut()
            && (playing.handle.state() == PlaybackState::Playing || playing.handle.state() == PlaybackState::Stopping)
        {
            if playing.track_name.as_str() == track_name {
                return;
            }

            if playing.handle.state() == PlaybackState::Playing {
                playing.handle.stop(Tween {
                    duration: Duration::from_secs(1),
                    ..Default::default()
                });
            }

            self.queued_background_music_track = Some(track_name.to_string());
            return;
        }

        self.change_background_music_track(track_name);
    }

    fn play_sound_effect(&mut self, sound_effect_key: SoundEffectKey) {
        if let Some(data) = self
            .cache
            .get(&sound_effect_key)
            .map(|cached_sound_effect| cached_sound_effect.0.clone())
        {
            let data = data.output_destination(&self.sound_effect_track);
            if let Err(_error) = self.manager.play(data.clone()) {
                #[cfg(feature = "debug")]
                print_debug!("[{}] can't play sound effect: {:?}", "error".red(), _error);
            }

            return;
        }

        queue_sound_effect_playback(
            self.game_file_loader.clone(),
            self.async_response_sender.clone(),
            &self.sound_effect_paths,
            &mut self.queued_sound_effect,
            sound_effect_key,
            None,
        );
    }

    fn set_ambient_listener(&mut self, position: Point3<f32>, view_direction: Vector3<f32>, look_up: Vector3<f32>) {
        let listener = Sphere::new(position, 10.0);

        self.query_result.clear();
        self.object_kdtree.query(&listener, &mut self.query_result);
        self.query_result.sort_unstable();

        // Add ambient sound that came into reach.
        difference(&mut self.query_result, &mut self.previous_query_result, &mut self.scratchpad);

        for ambient_key in self.scratchpad.iter().copied() {
            let Some(sound_config) = self.ambient_sound.get(ambient_key) else {
                #[cfg(feature = "debug")]
                print_debug!("[{}] can't find sound config for: {:?}", "error".red(), ambient_key);
                continue;
            };

            // Kira uses a RH coordinate system, so we need to convert our LH vectors.
            let position = sound_config.bounds.center();
            let position = Vector3::new(position.x, position.y, -position.z);
            let emitter_settings = EmitterSettings {
                distances: EmitterDistances {
                    min_distance: 5.0,
                    max_distance: sound_config.bounds.radius(),
                },
                attenuation_function: Some(Easing::Linear),
                enable_spatialization: true,
                persist_until_sounds_finish: true,
            };
            let emitter_handle = match self.scene.add_emitter(position, emitter_settings) {
                Ok(emitter_handle) => emitter_handle,
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] can't add ambient sound emitter: {:?}", "error".red(), _error);
                    continue;
                }
            };

            let sound_effect_key = sound_config.sound_effect_key;
            if let Some(data) = self
                .cache
                .get(&sound_effect_key)
                .map(|cached_sound_effect| cached_sound_effect.0.clone())
            {
                let data = adjust_ambient_sound(data, &emitter_handle, sound_config);
                match self.manager.play(data.clone()) {
                    Ok(handle) => {
                        if let Some(cycle) = sound_config.cycle {
                            self.cycling_ambient.insert(ambient_key, PlayingAmbient {
                                data,
                                handle,
                                cycle,
                                last_start: Instant::now(),
                            });
                        }
                    }
                    Err(_error) => {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] can't ambient sound effect: {:?}", "error".red(), _error);
                    }
                }
            } else {
                queue_sound_effect_playback(
                    self.game_file_loader.clone(),
                    self.async_response_sender.clone(),
                    &self.sound_effect_paths,
                    &mut self.queued_sound_effect,
                    sound_effect_key,
                    Some(ambient_key),
                );
            }

            self.active_emitters.insert(ambient_key, emitter_handle);
        }

        // Remove ambient sound that are out of reach.
        difference(&mut self.previous_query_result, &mut self.query_result, &mut self.scratchpad);
        for ambient_key in self.scratchpad.iter() {
            let _ = self.active_emitters.remove(ambient_key);
            let _ = self.cycling_ambient.remove(ambient_key);
        }

        // Update the previous result.
        swap(&mut self.query_result, &mut self.previous_query_result);

        // We only update the listener position once every 50 ms, so that we can
        // properly ease the change and have no discontinuities.
        let now = Instant::now();
        if now.duration_since(self.last_listener_update).as_secs_f32() > 0.05 {
            self.last_listener_update = now;

            // Kira uses a RH coordinate system, so we need to convert our LH vectors.
            let position = Vector3::new(position.x, position.y, -position.z);
            let view_direction = Vector3::new(view_direction.x, view_direction.y, -view_direction.z).normalize();
            let look_up = Vector3::new(look_up.x, look_up.y, -look_up.z).normalize();
            let right = view_direction.cross(look_up).normalize();
            let up = right.cross(view_direction);

            let rotation_matrix = Matrix3::from_cols(right, up, -view_direction);
            let orientation = Quaternion::from(rotation_matrix);

            let tween = Tween {
                duration: Duration::from_millis(50),
                ..Default::default()
            };
            self.ambient_listener.set_position(position, tween);
            self.ambient_listener.set_orientation(orientation, tween);
        }
    }

    fn add_ambient_sound(
        &mut self,
        sound_effect_key: SoundEffectKey,
        position: Point3<f32>,
        range: f32,
        volume: f32,
        cycle: Option<f32>,
    ) -> AmbientKey {
        self.ambient_sound
            .insert(AmbientSoundConfig {
                sound_effect_key,
                bounds: Sphere::new(position, range),
                volume,
                cycle,
            })
            .expect("Ambient sound slab is full")
    }

    fn remove_ambient_sound(&mut self, ambient_key: AmbientKey) {
        let _ = self.ambient_sound.remove(ambient_key);
        if let Some(emitter) = self.active_emitters.remove(&ambient_key) {
            // An emitter is removed from the spatial scene by dropping it. We make this
            // explicit to express our intent.
            drop(emitter);
        }
    }

    fn clear_ambient_sound(&mut self) {
        self.query_result.clear();
        self.previous_query_result.clear();
        self.scratchpad.clear();

        self.ambient_sound.clear();
        self.active_emitters.clear();
        self.cycling_ambient.clear();
    }

    fn prepare_ambient_sound_world(&mut self) {
        let objects: Vec<(AmbientKey, Sphere)> = self.ambient_sound.iter().map(|(key, object)| (key, object.bounds)).collect();

        if !objects.is_empty() {
            self.object_kdtree = KDTree::from_objects(&objects);
        }
    }

    fn update(&mut self) {
        self.resolve_async_loads();
        self.resolve_queued_audio();
        self.restart_cycling_ambient();
    }

    /// Audio engine will collect all static sound_effect data that finished
    /// loading. Should be called once a frame.
    fn resolve_async_loads(&mut self) {
        while let Ok(result) = self.async_response_receiver.try_recv() {
            match result {
                AsyncLoadResult::Loaded {
                    path: _path,
                    key,
                    sound_effect,
                } => {
                    self.loading_sound_effect.remove(&key);

                    if let Err(_error) = self.cache.insert(key, CachedSoundEffect(*sound_effect)) {
                        #[cfg(feature = "debug")]
                        print_debug!(
                            "[{}] audio file is too big for cache. Path: '{}': {:?}",
                            "error".red(),
                            &_path,
                            _error
                        );
                    }
                }
                AsyncLoadResult::Error {
                    path: _path,
                    key,
                    message: _message,
                } => {
                    self.loading_sound_effect.remove(&key);

                    #[cfg(feature = "debug")]
                    print_debug!(
                        "[{}] could not load audio file. Path: '{}' : {}",
                        "error".red(),
                        _path,
                        _message
                    );
                }
            }
        }
    }

    fn resolve_queued_audio(&mut self) {
        if self.queued_background_music_track.is_some()
            && let Some(playing) = self.current_background_music_track.as_ref()
            && playing.handle.state() == PlaybackState::Stopped
        {
            let track_name = self.queued_background_music_track.take().unwrap();
            self.change_background_music_track(&track_name)
        }

        let now = Instant::now();

        self.queued_sound_effect.retain(|queued| {
            if queued.queued_time.duration_since(now).as_secs_f32() > MAX_QUEUE_TIME_SECONDS {
                // We waited too long.
                return false;
            }

            let Some(data) = self
                .cache
                .get(&queued.sound_effect_key)
                .map(|cached_sound_effect| cached_sound_effect.0.clone())
            else {
                // Sound effect not loaded yet.
                return true;
            };

            match queued.ambient {
                None => {
                    if let Err(_error) = self.manager.play(data.output_destination(&self.sound_effect_track)) {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] can't play sound effect: {:?}", "error".red(), _error);
                    }
                }
                Some(ambient_key) => {
                    if let Some(emitter_handle) = self.active_emitters.get(&ambient_key)
                        && let Some(sound_config) = self.ambient_sound.get(ambient_key)
                    {
                        let data = adjust_ambient_sound(data, emitter_handle, sound_config);
                        match self.manager.play(data.clone()) {
                            Ok(handle) => {
                                if let Some(cycle) = sound_config.cycle {
                                    self.cycling_ambient.insert(ambient_key, PlayingAmbient {
                                        data,
                                        handle,
                                        cycle,
                                        last_start: Instant::now(),
                                    });
                                }
                            }
                            Err(_error) => {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] can't play ambient sound effect: {:?}", "error".red(), _error);
                            }
                        }
                    }
                }
            }

            // We played or can't play it.
            false
        });
    }

    fn restart_cycling_ambient(&mut self) {
        let now = Instant::now();

        for (_, playing) in self.cycling_ambient.iter_mut().filter(|(_, playing)| {
            playing.handle.state() != PlaybackState::Playing && now.duration_since(playing.last_start).as_secs_f32() >= playing.cycle
        }) {
            playing.last_start = now;

            match self.manager.play(playing.data.clone()) {
                Ok(handle) => {
                    playing.handle = handle;
                }
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] can't play ambient sound effect: {:?}", "error".red(), _error);
                }
            }
        }
    }

    fn change_background_music_track(&mut self, track_name: &str) {
        let Some(path) = find_file_path(track_name) else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] can't find background music track: {:?}", "error".red(), track_name);
            return;
        };

        let data = match StreamingSoundData::from_file(path) {
            Ok(sound_effect_data) => sound_effect_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!("[{}] can't decode background music track: {:?}", "error".red(), _error);
                return;
            }
        };

        // TODO: NHA Remove volume offset once we have a proper volume control in place.
        let data = data
            .volume(Volume::Amplitude(0.1))
            .output_destination(&self.background_music_track)
            .loop_region(..);
        let handle = match self.manager.play(data) {
            Ok(handle) => handle,
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!("[{}] can't play background music track: {:?}", "error".red(), _error);
                return;
            }
        };

        self.current_background_music_track = Some(BackgroundMusicTrack {
            track_name: track_name.to_string(),
            handle,
        });
    }
}

fn adjust_ambient_sound(mut data: StaticSoundData, emitter_handle: &EmitterHandle, sound_config: &AmbientSoundConfig) -> StaticSoundData {
    // Kira does the volume mapping from linear to logarithmic for us.
    data.settings.volume = Volume::Amplitude(sound_config.volume as f64).into();
    data.output_destination(emitter_handle)
}

fn queue_sound_effect_playback(
    game_file_loader: Arc<impl FileLoader>,
    async_response_sender: Sender<AsyncLoadResult>,
    sound_effect_paths: &GenerationalSlab<SoundEffectKey, String>,
    queued_sound_effect: &mut Vec<QueuedSoundEffect>,
    sound_effect_key: SoundEffectKey,
    ambient: Option<AmbientKey>,
) -> bool {
    let Some(path) = sound_effect_paths.get(sound_effect_key).cloned() else {
        // This case could happen, if the sound effect was queued for deletion.
        return true;
    };

    queued_sound_effect.push(QueuedSoundEffect {
        sound_effect_key,
        ambient,
        queued_time: Instant::now(),
    });

    spawn_async_load(game_file_loader, async_response_sender, path, sound_effect_key);
    false
}

/// Spawns a loading task on the standard thread pool.
fn spawn_async_load(
    game_file_loader: Arc<impl FileLoader>,
    async_response_sender: Sender<AsyncLoadResult>,
    path: String,
    key: SoundEffectKey,
) {
    spawn(move || {
        let full_path = format!("{SOUND_EFFECT_BASE_PATH}\\{path}");

        let data = match game_file_loader.get(&full_path) {
            Ok(data) => data,
            Err(error) => {
                let message = format!("can't find audio file: {error:?}");
                let _ = async_response_sender.send(AsyncLoadResult::Error { message, path, key });
                return;
            }
        };
        let sound_effect = match StaticSoundData::from_cursor(Cursor::new(data)) {
            Ok(sound_effect) => Box::new(sound_effect),
            Err(error) => {
                let message = format!("can't decode audio file: {error:?}");
                let _ = async_response_sender.send(AsyncLoadResult::Error { message, path, key });
                return;
            }
        };
        let _ = async_response_sender.send(AsyncLoadResult::Loaded { path, key, sound_effect });
    });
}

fn parse_background_music_track_mapping(game_file_loader: &impl FileLoader) -> HashMap<String, String> {
    let mut background_music_track_mapping: HashMap<String, String> = HashMap::new();

    match game_file_loader.get(BACKGROUND_MUSIC_MAPPING_FILE) {
        Ok(mapping_file_data) => {
            let content = String::from_utf8_lossy(&mapping_file_data);
            for line in content.lines() {
                if line.starts_with("//") {
                    continue;
                }
                let split: Vec<&str> = line.split('#').collect();
                if split.len() > 2 {
                    let resource_name = split[0].to_string();
                    let track_name = split[1].to_string();
                    background_music_track_mapping.insert(resource_name, track_name);
                }
            }
        }
        Err(_error) => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] can't find background music mapping file: {:?}", "error".red(), _error);
        }
    }

    background_music_track_mapping
}

fn find_file_path(path: &str) -> Option<PathBuf> {
    let path = match cfg!(target_os = "windows") {
        true => PathBuf::from(path),
        false => PathBuf::from(path.replace('\\', "/")),
    };

    #[cfg(feature = "flac")]
    let extensions = ["flac", "mp3", "wav"];

    #[cfg(not(feature = "flac"))]
    let extensions = ["mp3", "wav"];

    extensions.into_iter().find_map(|extension| {
        let mut new_path = path.clone();
        new_path.set_extension(extension);
        find_case_insensitive(&new_path)
    })
}

fn find_case_insensitive(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_string_lossy();
    let Ok(parent) = std::fs::read_dir(path.parent()?) else {
        return None;
    };

    parent
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().eq_ignore_ascii_case(&file_name))
        .map(|entry| entry.path())
}

fn difference<T: Ord + Copy>(vector_1: &mut [T], vector_2: &mut [T], result: &mut Vec<T>) {
    result.clear();

    let mut i = 0;
    let mut j = 0;

    while i < vector_1.len() && j < vector_2.len() {
        match vector_1[i].cmp(&vector_2[j]) {
            Ordering::Less => {
                result.push(vector_1[i]);
                i += 1;
            }
            Ordering::Equal => {
                i += 1;
                j += 1;
            }
            Ordering::Greater => {
                j += 1;
            }
        }
    }

    result.extend_from_slice(&vector_1[i..]);
}

#[cfg(test)]
mod tests {
    use crate::difference;

    #[test]
    fn test_difference() {
        let mut vector_1 = vec![1, 3, 4, 6, 7];
        let mut vector_2 = vec![2, 3, 5, 7, 8];
        let mut result = Vec::new();

        difference(&mut vector_1, &mut vector_2, &mut result);

        assert_eq!(result, vec![1, 4, 6]);
    }

    #[test]
    fn test_completely_different() {
        let mut vector_1 = vec![1, 3, 5];
        let mut vector_2 = vec![2, 4, 6];
        let mut result = Vec::new();

        difference(&mut vector_1, &mut vector_2, &mut result);

        assert_eq!(result, vec![1, 3, 5]);
    }

    #[test]
    fn test_one_empty_vector() {
        let mut vector_1 = vec![1, 2, 3];
        let mut vector_2: Vec<u32> = Vec::new();
        let mut result = Vec::new();

        difference(&mut vector_1, &mut vector_2, &mut result);

        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_no_difference() {
        let mut vector_1 = vec![1, 2, 3];
        let mut vector_2 = vec![1, 2, 3];
        let mut result = Vec::new();

        difference(&mut vector_1, &mut vector_2, &mut result);

        assert!(result.is_empty());
    }
}
