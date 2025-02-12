use cgmath::{Deg, InnerSpace, Matrix3, Rad, Vector3};
use num::Signed;
use ragnarok_formats::map::LightSettings;

use crate::graphics::Color;
use crate::settings::LightingMode;
use crate::system::{GAME_TIME_DAY_CYCLE, GAME_TIME_SCALE};

/// The angle the sun moves per game time second.
const SUN_ANGLE_PER_SECOND: f32 = (std::f64::consts::TAU / GAME_TIME_DAY_CYCLE) as f32;
const UPDATE_INTERVAL: f32 = SUN_ANGLE_PER_SECOND * GAME_TIME_SCALE as f32;
const HOUR_RADIANS: f32 = std::f32::consts::TAU / 24.0;
const SUN_PHASE_SHIFT: f32 = -HOUR_RADIANS * 6.0;
const MOON_PHASE_SHIFT: f32 = SUN_PHASE_SHIFT + HOUR_RADIANS * 12.0;
const TWILIGHT_HEIGHT: f32 = HOUR_RADIANS;
const MOONLIGHT_COLOR: Color = Color::rgb_u8(150, 150, 255);

pub struct Lighting {
    ambient_color: Color,
    diffuse_color: Color,
    light_latitude: f32,
    light_longitude: f32,
}

impl Lighting {
    pub fn new(settings: LightSettings) -> Self {
        let mut ambient_color: Color = settings.ambient_color.unwrap().into();

        // Workaround for map files with broken ambient color, where the ambient light
        // outshines shadows (for example "yuno").
        if ambient_color == Color::WHITE {
            // Ambient color values of "prontera".
            ambient_color = Color::rgb(0.55, 0.5, 0.5);
        }

        Self {
            ambient_color,
            diffuse_color: settings.diffuse_color.unwrap().into(),
            light_latitude: settings.light_latitude.unwrap() as f32,
            light_longitude: settings.light_longitude.unwrap() as f32,
        }
    }

    pub fn ambient_light_color(&self, lighting_mode: LightingMode, day_timer: f32) -> Color {
        match lighting_mode {
            LightingMode::Classic => self.ambient_color,
            LightingMode::Enhanced => {
                let sun_curve = Self::get_ambient_light_factor(day_timer, SUN_PHASE_SHIFT);
                let moon_curve = Self::get_ambient_light_factor(day_timer, MOON_PHASE_SHIFT);

                let day_ambient = Vector3::new(0.65, 0.65, 0.65);
                let night_ambient = Vector3::new(0.15, 0.15, 0.25);

                let ambient_mix = day_ambient * sun_curve + night_ambient * moon_curve;
                let ambient_channels = ambient_mix * 255.0;

                Self::color_from_channel(self.ambient_color, ambient_channels)
            }
        }
    }

    pub fn directional_light(&self, lighting_mode: LightingMode, day_timer: f32) -> (Vector3<f32>, Color) {
        match lighting_mode {
            LightingMode::Classic => {
                let rotation_around_x = Matrix3::from_angle_x(Deg(-self.light_latitude));
                let rotation_around_y = Matrix3::from_angle_y(Deg(self.light_longitude));
                let light_direction = rotation_around_y * (rotation_around_x * Vector3::new(0.0, 1.0, 0.0));

                (light_direction, self.diffuse_color)
            }
            LightingMode::Enhanced => {
                let (sun_angle, moon_angle) = Self::calculate_celestial_angles(day_timer);

                let (sun_direction, sun_intensity) = Self::calculate_celestial_direction(sun_angle);
                let sunlight_color = self.calculate_celestial_color::<true>(sun_angle, sun_intensity);

                let (moon_direction, moon_intensity) = Self::calculate_celestial_direction(moon_angle);
                let moonlight_color = self.calculate_celestial_color::<false>(moon_angle, moon_intensity);

                let (sun_factor, moon_factor) = Self::calculate_mixing_factors(sun_intensity, moon_intensity);
                let direction = (sun_direction * sun_factor + moon_direction * moon_factor).normalize();
                let color = sunlight_color * sun_factor + moonlight_color * moon_factor;

                (direction, color)
            }
        }
    }

    fn calculate_celestial_angles(day_timer: f32) -> (Rad<f32>, Rad<f32>) {
        let base_angle = day_timer * SUN_ANGLE_PER_SECOND;

        let sun_angle = ((base_angle + SUN_PHASE_SHIFT) / UPDATE_INTERVAL).floor() * UPDATE_INTERVAL;
        let moon_angle = ((base_angle + MOON_PHASE_SHIFT) / UPDATE_INTERVAL).floor() * UPDATE_INTERVAL;

        (Rad(sun_angle), Rad(moon_angle))
    }

    fn calculate_celestial_direction(angle: Rad<f32>) -> (Vector3<f32>, f32) {
        let raw_height = angle.0.sin();

        // Scale and then clamp minimal height above the horizon
        let clamped_height = (raw_height * 1.5).max(0.65);
        let direction = Vector3::new(angle.0.cos(), clamped_height, -angle.0.sin());

        // Smooth transition around horizon
        let intensity = if raw_height.abs() < TWILIGHT_HEIGHT {
            let progress = (raw_height / TWILIGHT_HEIGHT + 1.0) * 0.5;
            progress.clamp(0.0, 1.0).powi(3)
        } else if raw_height.is_positive() {
            1.0
        } else {
            0.0
        };

        (direction.normalize(), intensity)
    }

    fn calculate_celestial_color<const IS_SUNLIGHT: bool>(&self, angle: Rad<f32>, intensity: f32) -> Color {
        let height = angle.0.sin();

        match IS_SUNLIGHT {
            true => {
                let mut color: Color = self.diffuse_color;

                // Redshift during dawn/dusk
                if height.abs() < TWILIGHT_HEIGHT {
                    let red_shift = 1.0 - (height.abs() / TWILIGHT_HEIGHT);
                    color.red *= 1.0 + red_shift * 1.0;
                    color.green *= 1.0 - red_shift * 0.2;
                    color.blue *= 1.0 - red_shift * 0.3;
                }

                color * intensity
            }
            false => MOONLIGHT_COLOR * intensity,
        }
    }

    fn calculate_mixing_factors(sun_intensity: f32, moon_intensity: f32) -> (f32, f32) {
        if sun_intensity > moon_intensity {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        }
    }

    fn get_ambient_light_factor(day_timer: f32, phase: f32) -> f32 {
        let angle = day_timer * SUN_ANGLE_PER_SECOND + phase;
        let height = angle.sin();
        (height + 1.0) * 0.5
    }

    fn color_from_channel(base_color: Color, channels: Vector3<f32>) -> Color {
        Color::rgb_u8(
            (base_color.red * channels.x) as u8,
            (base_color.green * channels.y) as u8,
            (base_color.blue * channels.z) as u8,
        )
    }
}
