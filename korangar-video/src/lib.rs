//! This crate implements the AV1 video decoding.
//!
//! This code is a port of the `dav1d-rs` crate to use `rav1d`.
//! `rav1d` currently doesn't expose a safe Rust API, but will most likely
//! re-implement the API of `dav1d-rs`.
//!
//! The unsafe code in this crate is only needed because of the C-API that
//! currently `rav1d` exposes.
//!
//! `dav1d-rs` is also licensed under MIT.

/// Implements the IVF file format.
pub mod ivf;

use std::ffi::{c_int, c_void};
use std::{mem, ptr};

use rav1d::include::dav1d::data::*;
use rav1d::include::dav1d::dav1d::*;
pub use rav1d::include::dav1d::headers;
use rav1d::include::dav1d::picture::*;
use rav1d::send_sync_non_null::SendSyncNonNull;
use rav1d::*;

const fn dav1d_err(errno: c_int) -> c_int {
    if libc::EPERM < 0 { errno } else { -errno }
}

// Rav1dError is not exported, so we define the error value ourselves.
const EAGAIN: c_int = dav1d_err(libc::EAGAIN);
const EINVAL: c_int = dav1d_err(libc::EINVAL);
const ENOMEM: c_int = dav1d_err(libc::ENOMEM);
const ENOPROTOOPT: c_int = dav1d_err(libc::ENOPROTOOPT);

/// Error enum return by various `rav1d` operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Try again.
    ///
    /// If this is returned by [`Decoder::send_data`] or
    /// [`Decoder::send_pending_data`] then there are decoded frames pending
    /// that first have to be retrieved via [`Decoder::get_picture`]
    /// before processing any further pending data.
    ///
    /// If this is returned by [`Decoder::get_picture`] then no decoded frames
    /// are pending currently and more data needs to be sent to the decoder.
    Again,
    /// Invalid argument.
    ///
    /// One of the arguments passed to the function was invalid.
    InvalidArgument,
    /// Not enough memory.
    ///
    /// Not enough memory is currently available for performing this operation.
    NotEnoughMemory,
    /// Unsupported bitstream.
    ///
    /// The provided bitstream is not supported by `rav1d`.
    UnsupportedBitstream,
    /// Unknown error.
    UnknownError(i32),
}

impl From<i32> for Error {
    fn from(error: i32) -> Self {
        assert!(error < 0);

        match error {
            EAGAIN => Error::Again,
            EINVAL => Error::InvalidArgument,
            ENOMEM => Error::NotEnoughMemory,
            ENOPROTOOPT => Error::UnsupportedBitstream,
            _ => Error::UnknownError(error),
        }
    }
}

impl Error {
    pub const fn is_again(&self) -> bool {
        matches!(self, Error::Again)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Again => write!(fmt, "Try again"),
            Error::InvalidArgument => write!(fmt, "Invalid argument"),
            Error::NotEnoughMemory => write!(fmt, "Not enough memory available"),
            Error::UnsupportedBitstream => write!(fmt, "Unsupported bitstream"),
            Error::UnknownError(err) => write!(fmt, "Unknown error {err}"),
        }
    }
}

impl std::error::Error for Error {}

/// Settings for creating a new [`Decoder`] instance.
pub struct Settings {
    dav1d_settings: Dav1dSettings,
}

unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    /// Creates a new [`Settings`] instance with default settings.
    pub fn new() -> Self {
        unsafe {
            let mut dav1d_settings: mem::MaybeUninit<Dav1dSettings> = mem::MaybeUninit::uninit();
            dav1d_default_settings(ptr::NonNull::new_unchecked(dav1d_settings.as_mut_ptr()));
            Self {
                dav1d_settings: dav1d_settings.assume_init(),
            }
        }
    }

    pub fn set_n_threads(&mut self, n_threads: u32) {
        self.dav1d_settings.n_threads = n_threads as i32;
    }

    pub fn get_n_threads(&self) -> u32 {
        self.dav1d_settings.n_threads as u32
    }

    pub fn set_max_frame_delay(&mut self, max_frame_delay: u32) {
        self.dav1d_settings.max_frame_delay = max_frame_delay as i32;
    }

    pub fn get_max_frame_delay(&self) -> u32 {
        self.dav1d_settings.max_frame_delay as u32
    }

    pub fn set_apply_grain(&mut self, apply_grain: bool) {
        self.dav1d_settings.apply_grain = i32::from(apply_grain);
    }

    pub fn get_apply_grain(&self) -> bool {
        self.dav1d_settings.apply_grain != 0
    }

    pub fn set_operating_point(&mut self, operating_point: u32) {
        self.dav1d_settings.operating_point = operating_point as i32;
    }

    pub fn get_operating_point(&self) -> u32 {
        self.dav1d_settings.operating_point as u32
    }

    pub fn set_all_layers(&mut self, all_layers: bool) {
        self.dav1d_settings.all_layers = i32::from(all_layers);
    }

    pub fn get_all_layers(&self) -> bool {
        self.dav1d_settings.all_layers != 0
    }

    pub fn set_frame_size_limit(&mut self, frame_size_limit: u32) {
        self.dav1d_settings.frame_size_limit = frame_size_limit;
    }

    pub fn get_frame_size_limit(&self) -> u32 {
        self.dav1d_settings.frame_size_limit
    }

    pub fn set_strict_std_compliance(&mut self, strict_std_compliance: bool) {
        self.dav1d_settings.strict_std_compliance = i32::from(strict_std_compliance);
    }

    pub fn get_strict_std_compliance(&self) -> bool {
        self.dav1d_settings.strict_std_compliance != 0
    }

    pub fn set_output_invisible_frames(&mut self, output_invisible_frames: bool) {
        self.dav1d_settings.output_invisible_frames = i32::from(output_invisible_frames);
    }

    pub fn get_output_invisible_frames(&self) -> bool {
        self.dav1d_settings.output_invisible_frames != 0
    }

    pub fn set_inloop_filters(&mut self, inloop_filters: InloopFilterType) {
        self.dav1d_settings.inloop_filters = inloop_filters.bits();
    }

    pub fn get_inloop_filters(&self) -> InloopFilterType {
        InloopFilterType::from_bits_truncate(self.dav1d_settings.inloop_filters)
    }

    pub fn set_decode_frame_type(&mut self, decode_frame_type: DecodeFrameType) {
        self.dav1d_settings.decode_frame_type = decode_frame_type.into();
    }

    pub fn get_decode_frame_type(&self) -> DecodeFrameType {
        DecodeFrameType::try_from(self.dav1d_settings.decode_frame_type).expect("Invalid Dav1dDecodeFrameType")
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
    pub struct InloopFilterType: u32 {
        const DEBLOCK = DAV1D_INLOOPFILTER_DEBLOCK;
        const CDEF = DAV1D_INLOOPFILTER_CDEF;
        const RESTORATION = DAV1D_INLOOPFILTER_RESTORATION;
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum DecodeFrameType {
    #[default]
    All,
    Reference,
    Intra,
    Key,
}

impl TryFrom<u32> for DecodeFrameType {
    type Error = TryFromEnumError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            DAV1D_DECODEFRAMETYPE_ALL => Ok(DecodeFrameType::All),
            DAV1D_DECODEFRAMETYPE_REFERENCE => Ok(DecodeFrameType::Reference),
            DAV1D_DECODEFRAMETYPE_INTRA => Ok(DecodeFrameType::Intra),
            DAV1D_DECODEFRAMETYPE_KEY => Ok(DecodeFrameType::Key),
            _ => Err(TryFromEnumError(())),
        }
    }
}

impl From<DecodeFrameType> for u32 {
    fn from(v: DecodeFrameType) -> u32 {
        match v {
            DecodeFrameType::All => DAV1D_DECODEFRAMETYPE_ALL,
            DecodeFrameType::Reference => DAV1D_DECODEFRAMETYPE_REFERENCE,
            DecodeFrameType::Intra => DAV1D_DECODEFRAMETYPE_INTRA,
            DecodeFrameType::Key => DAV1D_DECODEFRAMETYPE_KEY,
        }
    }
}

/// The error type returned when a conversion from a C enum fails.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromEnumError(());

impl std::fmt::Display for TryFromEnumError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("Invalid enum value")
    }
}

impl From<std::convert::Infallible> for TryFromEnumError {
    fn from(x: std::convert::Infallible) -> TryFromEnumError {
        match x {}
    }
}

impl std::error::Error for TryFromEnumError {}

/// A `rav1d` decoder instance.
pub struct Decoder {
    decoder: Dav1dContext,
    settings: Dav1dSettings,
    pending_data: Option<Dav1dData>,
}

unsafe extern "C" fn release_wrapped_data(_data: *const u8, cookie: Option<SendSyncNonNull<c_void>>) {
    if let Some(cookie) = cookie {
        drop(unsafe { cookie.into_box() });
    }
}

impl Decoder {
    /// Creates a new [`Decoder`] instance with given [`Settings`].
    pub fn with_settings(settings: Settings) -> Result<Self, Error> {
        unsafe {
            let mut decoder = mem::MaybeUninit::uninit();

            let result = dav1d_open(
                ptr::NonNull::new(decoder.as_mut_ptr()),
                ptr::NonNull::new(&settings.dav1d_settings as *const _ as *mut _),
            );

            if result.0 < 0 {
                return Err(Error::from(result.0));
            }

            Ok(Decoder {
                decoder: decoder.assume_init().unwrap(),
                settings: settings.dav1d_settings,
                pending_data: None,
            })
        }
    }

    /// Creates a new [`Decoder`] instance with the default settings.
    pub fn new() -> Result<Self, Error> {
        Self::with_settings(Settings::default())
    }

    /// Flush the decoder.
    ///
    /// This flushes all delayed frames in the decoder and clears the internal
    /// decoder state.
    ///
    /// All currently pending frames are available afterward via
    /// [`Decoder::get_picture`].
    pub fn flush(&mut self) {
        unsafe {
            dav1d_flush(self.decoder);
            if let Some(mut pending_data) = self.pending_data.take() {
                dav1d_data_unref(ptr::NonNull::new(&mut pending_data));
            }
        }
    }

    /// Send new AV1 data to the decoder.
    ///
    /// After this returned `Ok(())` or `Err(Error::Again)` there might be
    /// decoded frames available via [`Decoder::get_picture`].
    ///
    /// # Panics
    ///
    /// If a previous call returned [`Error::Again`] then this must not be
    /// called again until [`Decoder::send_pending_data`] has returned
    /// `Ok(())`.
    pub fn send_data<T: AsRef<[u8]> + Send + 'static>(
        &mut self,
        buffer: T,
        offset: Option<i64>,
        timestamp: Option<i64>,
        duration: Option<i64>,
    ) -> Result<(), Error> {
        assert!(self.pending_data.is_none(), "Have pending data that needs to be handled first");

        let buffer = Box::new(buffer);
        let slice = (*buffer).as_ref();
        let len = slice.len();

        unsafe {
            let mut data: Dav1dData = mem::zeroed();
            let _result = dav1d_data_wrap(
                ptr::NonNull::new(&mut data),
                ptr::NonNull::new(slice.as_ptr() as *mut _),
                len,
                Some(release_wrapped_data),
                Some(SendSyncNonNull::from_ref(&*(Box::into_raw(buffer) as *mut c_void))),
            );
            if let Some(offset) = offset {
                data.m.offset = offset as _;
            }
            if let Some(timestamp) = timestamp {
                data.m.timestamp = timestamp;
            }
            if let Some(duration) = duration {
                data.m.duration = duration;
            }

            let result = dav1d_send_data(Some(self.decoder), ptr::NonNull::new(&mut data));
            if result.0 < 0 {
                let error = Error::from(result.0);

                if error.is_again() {
                    self.pending_data = Some(data);
                } else {
                    dav1d_data_unref(ptr::NonNull::new(&mut data));
                }

                return Err(error);
            }

            if data.sz > 0 {
                self.pending_data = Some(data);
                return Err(Error::Again);
            }

            Ok(())
        }
    }

    /// Sends any pending data to the decoder.
    ///
    /// This has to be called after [`Decoder::send_data`] has returned
    /// `Err(Error::Again)` to consume any further pending data.
    ///
    /// After this returned `Ok(())` or `Err(Error::Again)` there might be
    /// decoded frames available via [`Decoder::get_picture`].
    pub fn send_pending_data(&mut self) -> Result<(), Error> {
        let mut data = match self.pending_data.take() {
            None => {
                return Ok(());
            }
            Some(data) => data,
        };

        unsafe {
            let result = dav1d_send_data(Some(self.decoder), ptr::NonNull::new(&mut data));
            if result.0 < 0 {
                let error = Error::from(result.0);

                if error.is_again() {
                    self.pending_data = Some(data);
                } else {
                    dav1d_data_unref(ptr::NonNull::new(&mut data));
                }

                return Err(error);
            }

            if data.sz > 0 {
                self.pending_data = Some(data);
                return Err(Error::Again);
            }

            Ok(())
        }
    }

    /// Get the next decoded frame from the decoder.
    ///
    /// If this returns `Err(Error::Again)` then further data has to be sent to
    /// the decoder before further decoded frames become available.
    ///
    /// To make most use of frame threading this function should only be called
    /// once per submitted input frame and not until it returns
    /// `Err(Error::Again)`. Calling it in a loop should only be done to
    /// drain all pending frames at the end.
    pub fn get_picture(&mut self, picture: Option<Picture>) -> Result<Picture, Error> {
        unsafe {
            let mut picture = picture.unwrap_or_else(|| Picture {
                inner: Dav1dPicture::default(),
            });

            let result = dav1d_get_picture(Some(self.decoder), ptr::NonNull::new(&mut picture.inner));

            if result.0 < 0 { Err(Error::from(result.0)) } else { Ok(picture) }
        }
    }

    /// Get the decoder delay.
    pub fn get_frame_delay(&self) -> u32 {
        unsafe { dav1d_get_frame_delay(ptr::NonNull::new(&self.settings as *const _ as *mut _)).0 as u32 }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            if let Some(mut pending_data) = self.pending_data.take() {
                dav1d_data_unref(ptr::NonNull::new(&mut pending_data));
            }
            dav1d_close(ptr::NonNull::new(&mut Some(self.decoder)));
        };
    }
}

unsafe impl Send for Decoder {}
unsafe impl Sync for Decoder {}

/// A decoded frame.
pub struct Picture {
    inner: Dav1dPicture,
}

unsafe impl Send for Picture {}
unsafe impl Sync for Picture {}

/// Pixel layout of a frame.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum PixelLayout {
    /// Monochrome.
    I400,
    /// 4:2:0 planar.
    I420,
    /// 4:2:2 planar.
    I422,
    /// 4:4:4 planar.
    I444,
}

/// Frame component.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum PlanarImageComponent {
    /// Y component.
    Y,
    /// U component.
    U,
    /// V component.
    V,
}

impl From<usize> for PlanarImageComponent {
    fn from(index: usize) -> Self {
        match index {
            0 => PlanarImageComponent::Y,
            1 => PlanarImageComponent::U,
            2 => PlanarImageComponent::V,
            _ => panic!("Invalid YUV index: {}", index),
        }
    }
}

impl From<PlanarImageComponent> for usize {
    fn from(component: PlanarImageComponent) -> Self {
        match component {
            PlanarImageComponent::Y => 0,
            PlanarImageComponent::U => 1,
            PlanarImageComponent::V => 2,
        }
    }
}

/// The YUV color range.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum YUVRange {
    Limited,
    Full,
}

/// A single plane of a decoded frame.
///
/// This can be used like a `&[u8]`.
pub struct Plane<'a>(&'a Picture, PlanarImageComponent);

impl AsRef<[u8]> for Plane<'_> {
    fn as_ref(&self) -> &[u8] {
        let (stride, height) = self.0.plane_data_geometry(self.1);
        unsafe { std::slice::from_raw_parts(self.0.plane_data_ptr(self.1) as *const u8, (stride * height) as usize) }
    }
}

impl std::ops::Deref for Plane<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// Number of bits per component.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BitsPerComponent(pub usize);

impl Picture {
    /// Writes out the RGBA8 data of the picture.
    pub fn write_rgba8(&self, target: &mut [u8]) {
        let width = self.width() as usize;
        let height = self.height() as usize;
        let wanted = width * height * 4;

        let stride_y = self.stride(PlanarImageComponent::Y) as usize;
        let stride_u = self.stride(PlanarImageComponent::U) as usize;
        let stride_v = self.stride(PlanarImageComponent::V) as usize;

        assert_eq!(self.pixel_layout(), PixelLayout::I420);
        assert_eq!(
            target.len(),
            wanted,
            "Target RGBA8 array does not match image dimensions. Wanted: {} * {} * 4 = {}, got {}",
            width,
            height,
            wanted,
            target.len()
        );

        let plane_y = self.plane(PlanarImageComponent::Y);
        let plane_u = self.plane(PlanarImageComponent::U);
        let plane_v = self.plane(PlanarImageComponent::V);

        for y in 0..height {
            for x in 0..width {
                let base_tgt = (y * width + x) * 4;
                let base_y = y * stride_y + x;
                let base_u = (y / 2 * stride_u) + (x / 2);
                let base_v = (y / 2 * stride_v) + (x / 2);

                let rgb_pixel = &mut target[base_tgt..base_tgt + 4];

                let y: f32 = plane_y[base_y].into();
                let u: f32 = plane_u[base_u].into();
                let v: f32 = plane_v[base_v].into();

                rgb_pixel[0] = 1.402f32.mul_add(v - 128.0, y) as u8;
                rgb_pixel[1] = 0.714f32.mul_add(-(v - 128.0), 0.344f32.mul_add(-(u - 128.0), y)) as u8;
                rgb_pixel[2] = 1.772f32.mul_add(u - 128.0, y) as u8;
                rgb_pixel[3] = 255;
            }
        }
    }

    /// Stride in pixels of the `component` for the decoded frame.
    pub fn stride(&self, component: PlanarImageComponent) -> u32 {
        let index = match component {
            PlanarImageComponent::Y => 0,
            _ => 1,
        };
        self.inner.stride[index] as u32
    }

    /// Raw pointer to the data of the `component` for the decoded frame.
    pub fn plane_data_ptr(&self, component: PlanarImageComponent) -> *mut c_void {
        let index: usize = component.into();
        self.inner.data[index].unwrap().as_ptr()
    }

    /// Plane geometry of the `component` for the decoded frame.
    ///
    /// This returns the stride and height.
    pub fn plane_data_geometry(&self, component: PlanarImageComponent) -> (u32, u32) {
        let height = match component {
            PlanarImageComponent::Y => self.height(),
            _ => match self.pixel_layout() {
                PixelLayout::I420 => self.height().div_ceil(2),
                PixelLayout::I400 | PixelLayout::I422 | PixelLayout::I444 => self.height(),
            },
        };
        (self.stride(component), height)
    }

    /// Plane data of the `component` for the decoded frame.
    pub fn plane(&self, component: PlanarImageComponent) -> Plane<'_> {
        Plane(self, component)
    }

    /// The bit depth of the plane data.
    ///
    /// This returns 8 or 16 for the underlying integer type used for the plane
    /// data.
    ///
    /// Check [`Picture::bits_per_component`] for the number of bits that are
    /// used.
    pub fn bit_depth(&self) -> usize {
        self.inner.p.bpc as usize
    }

    /// Bits used per component of the plane data.
    ///
    /// Check [`Picture::bit_depth`] for the number of storage bits.
    pub fn bits_per_component(&self) -> Option<BitsPerComponent> {
        self.inner.seq_hdr.as_ref().and_then(|seq_hdr| unsafe {
            match seq_hdr.as_ref().hbd {
                0 => Some(BitsPerComponent(8)),
                1 => Some(BitsPerComponent(10)),
                2 => Some(BitsPerComponent(12)),
                _ => None,
            }
        })
    }

    /// Width of the frame.
    pub fn width(&self) -> u32 {
        self.inner.p.w as u32
    }

    /// Height of the frame.
    pub fn height(&self) -> u32 {
        self.inner.p.h as u32
    }

    /// Pixel layout of the frame.
    pub fn pixel_layout(&self) -> PixelLayout {
        #[allow(non_upper_case_globals)]
        match self.inner.p.layout {
            headers::DAV1D_PIXEL_LAYOUT_I400 => PixelLayout::I400,
            headers::DAV1D_PIXEL_LAYOUT_I420 => PixelLayout::I420,
            headers::DAV1D_PIXEL_LAYOUT_I422 => PixelLayout::I422,
            headers::DAV1D_PIXEL_LAYOUT_I444 => PixelLayout::I444,
            _ => panic!("Unknown DAV1D_PIXEL_LAYOUT"),
        }
    }

    /// Timestamp of the frame.
    ///
    /// This is the same timestamp as the one provided to
    /// [`Decoder::send_data`].
    pub fn timestamp(&self) -> Option<i64> {
        let timestamp = self.inner.m.timestamp;
        if timestamp == i64::MIN { None } else { Some(timestamp) }
    }

    /// Duration of the frame.
    ///
    /// This is the same duration as the one provided to [`Decoder::send_data`]
    /// or `0` if none was provided.
    pub fn duration(&self) -> i64 {
        self.inner.m.duration
    }

    /// Offset of the frame.
    ///
    /// This is the same offset as the one provided to [`Decoder::send_data`] or
    /// `-1` if none was provided.
    pub fn offset(&self) -> i64 {
        self.inner.m.offset as _
    }

    /// Chromaticity coordinates of the source colour primaries.
    pub fn color_primaries(&self) -> headers::Dav1dColorPrimaries {
        unsafe { self.inner.seq_hdr.unwrap().as_ref().pri }
    }

    /// Transfer characteristics function.
    pub fn transfer_characteristic(&self) -> headers::Dav1dTransferCharacteristics {
        unsafe { self.inner.seq_hdr.unwrap().as_ref().trc }
    }

    /// Matrix coefficients used in deriving luma and chroma signals from the
    /// green, blue and red or X, Y and Z primaries.
    pub fn matrix_coefficients(&self) -> headers::Dav1dMatrixCoefficients {
        unsafe { self.inner.seq_hdr.unwrap().as_ref().mtrx }
    }

    /// YUV color range.
    pub fn color_range(&self) -> YUVRange {
        unsafe {
            match self.inner.seq_hdr.unwrap().as_ref().color_range {
                0 => YUVRange::Limited,
                _ => YUVRange::Full,
            }
        }
    }

    /// Sample position for subsampled chroma.
    pub fn chroma_sample_position(&self) -> headers::Dav1dChromaSamplePosition {
        unsafe { self.inner.seq_hdr.unwrap().as_ref().chr }
    }

    /// Content light level information.
    pub fn content_light(&self) -> Option<ContentLightLevel> {
        self.inner.content_light.map(|content_light| unsafe {
            ContentLightLevel {
                max_content_light_level: content_light.as_ref().max_content_light_level,
                max_frame_average_light_level: content_light.as_ref().max_frame_average_light_level,
            }
        })
    }

    /// Mastering display information.
    pub fn mastering_display(&self) -> Option<MasteringDisplay> {
        self.inner.mastering_display.map(|mastering_display| unsafe {
            MasteringDisplay {
                primaries: mastering_display.as_ref().primaries,
                white_point: mastering_display.as_ref().white_point,
                max_luminance: mastering_display.as_ref().max_luminance,
                min_luminance: mastering_display.as_ref().min_luminance,
            }
        })
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        unsafe {
            dav1d_picture_unref(ptr::NonNull::new(&mut self.inner));
        }
    }
}

/// Content light level information as specified in CEA-861.3, Appendix A.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentLightLevel {
    /// Maximum content light level (MaxCLL) in candela per square metre.
    pub max_content_light_level: u16,
    /// Maximum frame average light level (MaxFLL) in candela per square metre.
    pub max_frame_average_light_level: u16,
}

/// Mastering display information as specified in SMPTE ST 2086.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MasteringDisplay {
    /// Red/green/blue XY coordinates of primaries in CIE 1931 color space as
    /// 0.16 fixed-point number.
    pub primaries: [[u16; 2usize]; 3usize],
    /// XY coordinates of white point in CIE 1931 color space as 0.16
    /// fixed-point number.
    pub white_point: [u16; 2usize],
    /// Maximum luminance in candela per square metre as 24.8 fixed-point
    /// number.
    pub max_luminance: u32,
    /// Minimum luminance in candela per square metre as 18.14 fixed-point
    /// number.
    pub min_luminance: u32,
}

#[cfg(test)]
mod test {
    use crate::ivf::Ivf;

    static TEST_FILE_420_8: &[u8] = include_bytes!("../testfile/test-420-8.ivf");
    static TEST_FILE_420_12: &[u8] = include_bytes!("../testfile/test-420-12.ivf");

    fn handle_pending_pictures(decoder: &mut super::Decoder, pictures: &mut Vec<super::Picture>, drain: bool) {
        loop {
            match decoder.get_picture(None) {
                Ok(picture) => pictures.push(picture),
                // Need to send more data to the decoder before it can decode new pictures
                Err(error) if error.is_again() => return,
                Err(error) => panic!("Error getting decoded pictures: {}", error),
            }

            if !drain {
                break;
            }
        }
    }

    fn check_pictures(pictures: &[super::Picture], bpp: usize) {
        assert_eq!(pictures.len(), 5);

        let pts = [0, 33, 67, 100, 133];

        for (i, picture) in pictures.iter().enumerate() {
            assert_eq!(picture.width(), 320);
            assert_eq!(picture.height(), 240);
            assert_eq!(picture.bit_depth(), bpp);
            assert_eq!(picture.bits_per_component(), Some(super::BitsPerComponent(bpp)));
            assert_eq!(picture.pixel_layout(), super::PixelLayout::I420);
            assert_eq!(picture.color_primaries(), super::headers::DAV1D_COLOR_PRI_BT709);
            assert_eq!(picture.transfer_characteristic(), super::headers::DAV1D_TRC_BT709);
            assert_eq!(picture.matrix_coefficients(), super::headers::DAV1D_MC_BT709);
            assert_eq!(picture.chroma_sample_position(), super::headers::DAV1D_CHR_UNKNOWN);
            assert_eq!(picture.timestamp(), Some(pts[i]));
            assert_eq!(picture.offset(), i as i64);

            let stride_mult = if bpp == 8 { 1 } else { 2 };

            assert!(picture.stride(super::PlanarImageComponent::Y) >= 320 * stride_mult);
            assert!(picture.stride(super::PlanarImageComponent::U) >= 160 * stride_mult);
            assert!(picture.stride(super::PlanarImageComponent::V) >= 160 * stride_mult);

            assert_eq!(picture.plane_data_geometry(super::PlanarImageComponent::Y).1, 240);
            assert_eq!(picture.plane_data_geometry(super::PlanarImageComponent::U).1, 120);
            assert_eq!(picture.plane_data_geometry(super::PlanarImageComponent::V).1, 120);

            assert_eq!(
                picture.plane(super::PlanarImageComponent::Y).len(),
                picture.stride(super::PlanarImageComponent::Y) as usize * 240
            );

            assert_eq!(
                picture.plane(super::PlanarImageComponent::U).len(),
                picture.stride(super::PlanarImageComponent::U) as usize * 120
            );

            assert_eq!(
                picture.plane(super::PlanarImageComponent::V).len(),
                picture.stride(super::PlanarImageComponent::V) as usize * 120
            );
        }
    }

    fn decode_file(file: &[u8], mut decoder: super::Decoder, pictures: &mut Vec<super::Picture>) {
        use std::io;

        let reader = io::Cursor::new(file);

        let mut video = Ivf::new(reader).unwrap();
        let header = *video.header();

        println!("{:?}", header);

        let mut index = 0;

        while let Ok(Some(packet)) = video.read_frame() {
            println!("Packet {}", packet.timestamp);

            // Let's use millisecond timestamps
            let pts = 1000 * packet.timestamp as i64 * header.timebase_numerator as i64 / header.timebase_denominator as i64;

            // Send packet to the decoder
            match decoder.send_data(packet.packet, Some(index), Some(pts), None) {
                Err(error) if error.is_again() => {
                    // If the decoder did not consume all data, output all
                    // pending pictures and send pending data to the decoder
                    // until it is all used up.
                    loop {
                        handle_pending_pictures(&mut decoder, pictures, false);

                        match decoder.send_pending_data() {
                            Err(e) if e.is_again() => continue,
                            Err(e) => {
                                panic!("Error sending pending data to the decoder: {}", e);
                            }
                            _ => break,
                        }
                    }
                }
                Err(e) => {
                    panic!("Error sending data to the decoder: {}", e);
                }
                _ => (),
            }

            // Handle all pending pictures before sending the next data.
            handle_pending_pictures(&mut decoder, pictures, false);

            index += 1;
        }

        // Handle all pending pictures that were not output yet.
        handle_pending_pictures(&mut decoder, pictures, true);
    }

    #[test]
    fn test_basic_420_8() {
        let decoder = super::Decoder::new().expect("failed to create decoder instance");
        let mut pictures = vec![];
        decode_file(TEST_FILE_420_8, decoder, &mut pictures);
        check_pictures(&pictures, 8);
    }

    #[test]
    fn test_basic_420_12() {
        let decoder = super::Decoder::new().expect("failed to create decoder instance");
        let mut pictures = vec![];
        decode_file(TEST_FILE_420_12, decoder, &mut pictures);
        check_pictures(&pictures, 12);
    }

    #[test]
    fn test_allocator_420_8() {
        let decoder = super::Decoder::new().expect("failed to create decoder instance");
        let mut pictures = vec![];
        decode_file(TEST_FILE_420_8, decoder, &mut pictures);
        check_pictures(&pictures, 8);
        assert_eq!(pictures.len(), 5);
    }

    #[test]
    fn test_allocator_420_12() {
        let decoder = super::Decoder::new().expect("failed to create decoder instance");
        let mut pictures = vec![];
        decode_file(TEST_FILE_420_12, decoder, &mut pictures);
        check_pictures(&pictures, 12);
        assert_eq!(pictures.len(), 5);
    }
}
