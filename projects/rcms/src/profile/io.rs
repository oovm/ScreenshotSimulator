use super::{
    ColorSpace, DebugFmtTag, IccDataType, IccProfile, IccTag, IccTagData, IccValue, ProfileClass,
};
use crate::color::{CxyY, Cxyz};
use crate::fixed::{s15f16, u16f16, ReprError};
use crate::profile::mlu::Mlu;
use crate::tone_curve::ToneCurve;
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt, BE};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::io::{Read, Seek, Write};
use std::{fmt, io, mem};

/// A profile deserialization error.
#[derive(Debug)]
#[non_exhaustive]
pub enum DeserError {
    /// Magic number mismatch.
    Magic,
    /// The file is too big.
    TooBig,
    /// Unknown device class.
    UnknownDeviceClass(u32),
    /// Unknown color space.
    UnknownColorSpace(u32),
    /// Unsupported profile connection space.
    UnsupportedPCS(ColorSpace),
    /// Unknown rendering intent.
    UnknownIntent(u32),
    /// Invalid creation date.
    InvalidCreationDate,
    /// Duplicate tag.
    DuplicateTag(u32),
    /// Invalid tag pointer.
    InvalidTagPointer(u32),
    /// The type is unsupported for the given tag.
    UnsupportedData(IccTag, IccDataType),
    /// An error occurred while deserializing tag data.
    TagData(IccTag, IccDataType, DataDeserError),
    /// An IO error.
    Io(io::Error),
}

impl fmt::Display for DeserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeserError::Magic => write!(f, "incorrect magic number"),
            DeserError::TooBig => write!(f, "file is too big"),
            DeserError::UnknownDeviceClass(x) => {
                write!(f, "unknown device class {:?}", DebugFmtTag(*x))
            }
            DeserError::UnknownColorSpace(x) => {
                write!(f, "unknown color space {:?}", DebugFmtTag(*x))
            }
            DeserError::UnsupportedPCS(x) => write!(f, "unsupported PCS {}", x),
            DeserError::UnknownIntent(x) => {
                write!(f, "unknown rendering intent {:?}", DebugFmtTag(*x))
            }
            DeserError::InvalidCreationDate => write!(f, "invalid creation date"),
            DeserError::DuplicateTag(x) => write!(f, "duplicate tag {:?}", DebugFmtTag(*x)),
            DeserError::InvalidTagPointer(x) => {
                write!(f, "invalid tag pointer for tag {:?}", DebugFmtTag(*x))
            }
            DeserError::UnsupportedData(x, y) => {
                write!(f, "unsupported data type {:?} for tag {:?}", y, x)
            }
            DeserError::TagData(tag, ty, err) => write!(
                f,
                "error deserializing tag {:?} of type {:?}: {}",
                tag, ty, err
            ),
            DeserError::Io(err) => write!(f, "{}", err),
        }
    }
}

impl Error for DeserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DeserError::Io(err) => Some(err),
            _ => None,
        }
    }
}

/// A tag data deserialization error.
#[derive(Debug)]
#[non_exhaustive]
pub enum DataDeserError {
    /// Incorrect number of chromaticity channels.
    ChromaticityChannels(u16),
    /// Too many colorants.
    ColorantOrderCount(usize),
    /// Invalid tone curve.
    InvalidToneCurve,
    /// Unknown parametric curve type.
    UnknownParametricCurve(u16),
    /// Invalid MLU record size.
    InvalidMluRecordSize(u32),
    /// Invalid MLU data.
    InvalidMlu,
    /// An IO error.
    Io(io::Error),
}

impl fmt::Display for DataDeserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DataDeserError::ChromaticityChannels(x) => {
                write!(f, "chromaticity has incorrect number of channels ({})", x)
            }
            DataDeserError::ColorantOrderCount(x) => write!(f, "too many colorants ({})", x),
            DataDeserError::InvalidToneCurve => write!(f, "invalid tone curve"),
            DataDeserError::UnknownParametricCurve(x) => {
                write!(f, "unknown parametric curve type {}", x)
            }
            DataDeserError::InvalidMluRecordSize(x) => write!(f, "invalid MLU record size {}", x),
            DataDeserError::InvalidMlu => write!(f, "invalid MLU"),
            DataDeserError::Io(err) => write!(f, "{}", err),
        }
    }
}

impl Error for DataDeserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DataDeserError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for DeserError {
    fn from(err: io::Error) -> DeserError {
        DeserError::Io(err)
    }
}

impl From<io::Error> for DataDeserError {
    fn from(err: io::Error) -> DataDeserError {
        DataDeserError::Io(err)
    }
}

/// Serialization errors.
#[derive(Debug)]
pub enum SerError {
    /// Broken tag link. Tag (0) references tag (1) (possibly through several layers of indirection)
    /// but tag (1) does not exist.
    BrokenLink(u32, u32),
    /// Too much indirection while trying to evaluate this tag.
    TooMuchIndirection(u32),
    /// An IO error.
    Io(io::Error),
    /// Failed to encode a float as a fixed-point number.
    F64Repr(f64),
}

impl fmt::Display for SerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SerError::BrokenLink(tag, cursor) => write!(
                f,
                "broken link: {:?} referenced from {:?} does not exist",
                DebugFmtTag(*cursor),
                DebugFmtTag(*tag)
            ),
            SerError::TooMuchIndirection(tag) => {
                write!(f, "too much indirection in tag {:?}", DebugFmtTag(*tag))
            }
            SerError::Io(err) => write!(f, "{}", err),
            SerError::F64Repr(x) => write!(f, "could not encode {} as a fixed-point value", x),
        }
    }
}

impl Error for SerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SerError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for SerError {
    fn from(err: io::Error) -> SerError {
        SerError::Io(err)
    }
}

impl From<ReprError<f64>> for SerError {
    fn from(err: ReprError<f64>) -> SerError {
        SerError::F64Repr(err.0)
    }
}

/// Enforces that the profile version is per spec.
/// Operates on the big endian bytes from the profile.
/// Called before converting to platform endianness.
/// Byte 0 is BCD major version, so max 9.
/// Byte 1 is 2 BCD digits, one per nibble.
/// Reserved bytes 2 & 3 must be 0.
fn validated_version(version: u32) -> u32 {
    let mut bytes = version.to_be_bytes();
    if bytes[0] > 0x09 {
        bytes[0] = 0x09;
    }
    let mut tmp1 = bytes[1] & 0xf0;
    let mut tmp2 = bytes[1] & 0x0f;
    if tmp1 > 0x90 {
        tmp1 = 0x90;
    }
    if tmp2 > 0x09 {
        tmp2 = 0x09;
    }
    bytes[1] = tmp1 | tmp2;
    bytes[2] = 0;
    bytes[3] = 0;
    u32::from_be_bytes(bytes)
}

#[derive(Debug)]
enum TagTableEntry {
    Linked(u32),
    Offset(u32, u32),
}

impl IccProfile {
    const HEADER_SIZE: usize = 128;
    /// `acsp`
    const MAGIC: u32 = 0x61637370;

    fn read_header<T: io::Read>(&mut self, input: &mut T) -> Result<usize, DeserError> {
        let size = input.read_u32::<BE>()?;
        let preferred_cmm = input.read_u32::<BE>()?;
        let profile_version = input.read_u32::<BE>()?;
        let device_class = input.read_u32::<BE>()?;
        let color_space = input.read_u32::<BE>()?;
        let pcs = input.read_u32::<BE>()?;
        let creation_year = input.read_u16::<BE>()?;
        let creation_month = input.read_u16::<BE>()?;
        let creation_day = input.read_u16::<BE>()?;
        let creation_hours = input.read_u16::<BE>()?;
        let creation_minutes = input.read_u16::<BE>()?;
        let creation_seconds = input.read_u16::<BE>()?;
        let magic = input.read_u32::<BE>()?;
        let platform = input.read_u32::<BE>()?;
        let flags = input.read_u32::<BE>()?;
        let manufacturer = input.read_u32::<BE>()?;
        let model = input.read_u32::<BE>()?;
        let attributes = input.read_u64::<BE>()?;
        let intent = input.read_u32::<BE>()?;
        let _illuminant_x = input.read_u32::<BE>()?;
        let _illuminant_y = input.read_u32::<BE>()?;
        let _illuminant_z = input.read_u32::<BE>()?;
        let creator = input.read_u32::<BE>()?;
        let id = input.read_u128::<BE>()?;
        let mut reserved_bytes = [0; 28];
        input.read(&mut reserved_bytes)?;

        if magic != Self::MAGIC {
            return Err(DeserError::Magic);
        }

        self.device_class = device_class
            .try_into()
            .map_err(DeserError::UnknownDeviceClass)?;
        self.color_space = color_space
            .try_into()
            .map_err(DeserError::UnknownColorSpace)?;
        self.pcs = pcs.try_into().map_err(DeserError::UnknownColorSpace)?;
        match self.pcs {
            ColorSpace::XYZ | ColorSpace::Lab => (),
            _ => return Err(DeserError::UnsupportedPCS(self.pcs)),
        }
        self.flags = flags;
        self.manufacturer = manufacturer;
        self.model = model;
        self.attributes = attributes;
        self.rendering_intent = intent.try_into().map_err(DeserError::UnknownIntent)?;
        self.version = validated_version(profile_version);
        self.creator = creator;
        self.preferred_cmm = preferred_cmm;
        self.platform = platform;
        self.id = id;
        self.created = {
            let year = creation_year as i32;
            let month = creation_month
                .try_into()
                .map_err(|_| DeserError::InvalidCreationDate)?;
            let day = creation_day
                .try_into()
                .map_err(|_| DeserError::InvalidCreationDate)?;
            let hours = creation_hours
                .try_into()
                .map_err(|_| DeserError::InvalidCreationDate)?;
            let minutes = creation_minutes
                .try_into()
                .map_err(|_| DeserError::InvalidCreationDate)?;
            let seconds = creation_seconds
                .try_into()
                .map_err(|_| DeserError::InvalidCreationDate)?;

            time::PrimitiveDateTime::new(
                time::Date::try_from_ymd(year, month, day)
                    .map_err(|_| DeserError::InvalidCreationDate)?,
                time::Time::try_from_hms(hours, minutes, seconds)
                    .map_err(|_| DeserError::InvalidCreationDate)?,
            )
            .assume_offset(time::UtcOffset::UTC)
        };

        Ok(size as usize)
    }

    fn read_tag_table<T: io::Read>(
        input: &mut T,
    ) -> Result<HashMap<u32, TagTableEntry>, DeserError> {
        let tag_count = input.read_u32::<BE>()?;

        let mut tags = HashMap::new();
        let mut tags_by_ptr = HashMap::new();

        for _ in 0..tag_count {
            let tag = input.read_u32::<BE>()?;
            let ptr = input.read_u32::<BE>()?;
            let size = input.read_u32::<BE>()?;

            if tags.contains_key(&tag) {
                return Err(DeserError::DuplicateTag(tag));
            }

            if let Some(link) = tags_by_ptr.get(&ptr) {
                tags.insert(tag, TagTableEntry::Linked(*link));
            } else {
                tags.insert(tag, TagTableEntry::Offset(ptr, size));
                tags_by_ptr.insert(ptr, tag);
            }
        }

        Ok(tags)
    }

    /// Deserializes an ICC profile.
    pub fn deserialize<T: io::Read>(input: &mut T) -> Result<Self, DeserError> {
        // created with dummy values that’ll be filled in in read_header
        let mut profile = IccProfile::new(ProfileClass::Abstract, ColorSpace::XYZ);
        let profile_size = profile.read_header(input)?;
        let tag_table = Self::read_tag_table(input)?;

        // hard limit at 16 MiB
        if profile_size > 16_777_216 {
            return Err(DeserError::TooBig);
        }

        let tag_table_size = 4 + 12 * tag_table.len();

        let pool_size = profile_size - Self::HEADER_SIZE - tag_table_size;
        let mut pool = Vec::with_capacity(pool_size);
        pool.resize(pool_size, 0);
        input.read_exact(&mut pool)?;

        for (tag, pointer) in tag_table {
            match pointer {
                TagTableEntry::Linked(target) => {
                    profile.tags.insert(tag, IccTagData::Linked(target));
                }
                TagTableEntry::Offset(off, len) => {
                    if (off as usize) < Self::HEADER_SIZE {
                        return Err(DeserError::InvalidTagPointer(tag));
                    }

                    let offset = off as usize - Self::HEADER_SIZE - tag_table_size;
                    let mut end = offset + len as usize;

                    if end > pool_size {
                        return Err(DeserError::InvalidTagPointer(tag));
                    }

                    let data_type = BE::read_u32(&pool[offset..]);

                    let icc_tag = IccTag::try_from(tag).ok();
                    let icc_type = IccDataType::try_from(data_type).ok();

                    let known_types = icc_tag.map_or(None, tag_data_types);

                    if let (Some(icc_tag), Some(icc_type), Some(known_types)) =
                        (icc_tag, icc_type, known_types)
                    {
                        if let None = known_types.iter().find(|x| **x == icc_type) {
                            return Err(DeserError::UnsupportedData(icc_tag, icc_type));
                        }

                        // MLU size is supposedly not to be trusted
                        if icc_type == IccDataType::Mlu {
                            // just pass the whole thing
                            end = pool.len();
                        }

                        let data = &pool[(offset + 8)..end];

                        if let Some(deserialize) = tag_deserializer(icc_type) {
                            let deserialized = deserialize(data)
                                .map_err(|e| DeserError::TagData(icc_tag, icc_type, e))?;
                            profile.tags.insert(tag, IccTagData::Value(deserialized));
                        } else {
                            profile.tags.insert(tag, IccTagData::Raw(data.to_vec()));
                        }
                    } else {
                        let data = &pool[offset..end];
                        profile.tags.insert(tag, IccTagData::Raw(data.to_vec()));
                    }
                }
            }
        }

        Ok(profile)
    }

    fn write_header<T: io::Write>(&self, buf: &mut T, total_size: u32) -> Result<(), SerError> {
        buf.write_u32::<BE>(total_size)?;
        buf.write_u32::<BE>(self.preferred_cmm)?;
        buf.write_u32::<BE>(self.version)?;
        buf.write_u32::<BE>(self.device_class.into())?;
        buf.write_u32::<BE>(self.color_space.into())?;
        buf.write_u32::<BE>(self.pcs.into())?;

        {
            let utc_time = self.created.to_offset(time::UtcOffset::UTC);
            let date = utc_time.date();
            let time = utc_time.time();
            buf.write_u16::<BE>(date.year() as u16)?;
            buf.write_u16::<BE>(date.month() as u16)?;
            buf.write_u16::<BE>(date.day() as u16)?;
            buf.write_u16::<BE>(time.hour() as u16)?;
            buf.write_u16::<BE>(time.minute() as u16)?;
            buf.write_u16::<BE>(time.second() as u16)?;
        }

        buf.write_u32::<BE>(Self::MAGIC)?;
        buf.write_u32::<BE>(self.platform)?;
        buf.write_u32::<BE>(self.flags)?;
        buf.write_u32::<BE>(self.manufacturer)?;
        buf.write_u32::<BE>(self.model)?;
        buf.write_u64::<BE>(self.attributes)?;
        buf.write_u32::<BE>(self.rendering_intent.into())?;
        buf.write_i32::<BE>(s15f16::try_from(0.964)?.to_bytes())?;
        buf.write_i32::<BE>(s15f16::try_from(1.)?.to_bytes())?;
        buf.write_i32::<BE>(s15f16::try_from(0.8249)?.to_bytes())?;
        buf.write_u32::<BE>(self.creator)?;
        buf.write_u128::<BE>(self.id)?;

        // reserved bytes
        let reserved = [0; 28];
        buf.write_all(&reserved)?;

        Ok(())
    }

    /// Serializes this ICC profile.
    pub fn serialize<T: io::Write>(&self, output: &mut T) -> Result<(), SerError> {
        let mut tag_pool = HashMap::new();

        for (tag, data) in &self.tags {
            match data {
                IccTagData::Value(value) => {
                    let mut buf_inner = Vec::new();
                    let mut buf = io::BufWriter::new(&mut buf_inner);
                    value.serialize(&mut buf)?;
                    buf.flush()?;
                    drop(buf);
                    tag_pool.insert(tag, buf_inner);
                }
                IccTagData::Raw(raw) => {
                    tag_pool.insert(tag, raw.clone());
                }
                IccTagData::Linked(_) => (),
            }
        }

        let tags_size = 4 + 12 * self.tags.len() as u32;

        enum LayoutItem {
            Padding(u32),
            Tag(u32),
        }

        let mut layout = Vec::new();
        let mut pool_layout = HashMap::new();
        let mut layout_cursor = Self::HEADER_SIZE as u32 + tags_size;
        for (tag, data) in &tag_pool {
            // align to 32 bits
            let skip = (4 - (layout_cursor % 4)) % 4;
            if skip > 0 {
                layout.push(LayoutItem::Padding(skip));
            }
            layout_cursor += skip;

            layout.push(LayoutItem::Tag(**tag));
            pool_layout.insert(*tag, layout_cursor);
            layout_cursor += data.len() as u32;
        }

        let total_size = layout_cursor as u32;
        self.write_header(output, total_size)?;

        output.write_u32::<BE>(self.tags.len() as u32)?;

        for (tag, _) in &self.tags {
            output.write_u32::<BE>(*tag)?;

            let mut cursor = *tag;
            let mut depth = 0;
            loop {
                if depth > 30 {
                    Err(SerError::TooMuchIndirection(*tag))?
                }
                if let Some(data) = self.tags.get(&cursor) {
                    if let IccTagData::Linked(other) = data {
                        cursor = *other;
                        depth += 1;
                    } else {
                        break;
                    }
                } else {
                    Err(SerError::BrokenLink(*tag, cursor))?
                }
            }

            let offset = pool_layout.get(&cursor).expect("internal inconsistency");
            output.write_u32::<BE>(*offset)?;
            let data_size = tag_pool.get(&cursor).expect("internal inconsistency").len() as u32;
            output.write_u32::<BE>(data_size)?;
        }

        for item in layout {
            match item {
                LayoutItem::Padding(bytes) => {
                    for _ in 0..bytes {
                        output.write_u8(0)?;
                    }
                }
                LayoutItem::Tag(tag) => {
                    let data = tag_pool.get(&tag).unwrap();
                    output.write_all(data)?;
                }
            }
        }

        Ok(())
    }
}

fn tag_data_types(for_tag: IccTag) -> Option<&'static [IccDataType]> {
    type Tt = IccDataType;
    const TAG_DATA_TYPES: &[(IccTag, &[Tt])] = &[
        (IccTag::AToB0, &[Tt::Lut16, Tt::LutAToB, Tt::Lut8]),
        (IccTag::AToB1, &[Tt::Lut16, Tt::LutAToB, Tt::Lut8]),
        (IccTag::AToB2, &[Tt::Lut16, Tt::LutAToB, Tt::Lut8]),
        (IccTag::BToA0, &[Tt::Lut16, Tt::LutBToA, Tt::Lut8]),
        (IccTag::BToA1, &[Tt::Lut16, Tt::LutBToA, Tt::Lut8]),
        (IccTag::BToA2, &[Tt::Lut16, Tt::LutBToA, Tt::Lut8]),
        (IccTag::RedColorant, &[Tt::Xyz]),
        (IccTag::GreenColorant, &[Tt::Xyz]),
        (IccTag::BlueColorant, &[Tt::Xyz]),
        (IccTag::RedTRC, &[Tt::Curve, Tt::ParametricCurve]),
        (IccTag::GreenTRC, &[Tt::Curve, Tt::ParametricCurve]),
        (IccTag::BlueTRC, &[Tt::Curve, Tt::ParametricCurve]),
        (IccTag::CalibrationDateTime, &[Tt::DateTime]),
        (IccTag::CharTarget, &[Tt::Text]),
        (IccTag::ChromaticAdaptation, &[Tt::S15Fixed16Array]),
        (IccTag::Chromaticity, &[Tt::Chromaticity]),
        (IccTag::ColorantOrder, &[Tt::ColorantOrder]),
        (IccTag::ColorantTable, &[Tt::ColorantTable]),
        (IccTag::ColorantTableOut, &[Tt::ColorantTable]),
        (IccTag::Copyright, &[Tt::Text, Tt::Mlu, Tt::TextDescription]),
        (IccTag::DateTime, &[Tt::DateTime]),
        (
            IccTag::DeviceMfgDesc,
            &[Tt::TextDescription, Tt::Mlu, Tt::Text],
        ),
        (
            IccTag::DeviceModelDesc,
            &[Tt::TextDescription, Tt::Mlu, Tt::Text],
        ),
        (IccTag::Gamut, &[Tt::Lut16, Tt::LutBToA, Tt::Lut8]),
        (IccTag::GrayTRC, &[Tt::Curve, Tt::ParametricCurve]),
        (IccTag::Luminance, &[Tt::Xyz]),
        (IccTag::MediaBlackPoint, &[Tt::Xyz]),
        (IccTag::MediaWhitePoint, &[Tt::Xyz]),
        (IccTag::NamedColor2, &[Tt::NamedColor2]),
        (IccTag::Preview0, &[Tt::Lut16, Tt::LutBToA, Tt::Lut8]),
        (IccTag::Preview1, &[Tt::Lut16, Tt::LutBToA, Tt::Lut8]),
        (IccTag::Preview2, &[Tt::Lut16, Tt::LutBToA, Tt::Lut8]),
        (
            IccTag::ProfileDescription,
            &[Tt::TextDescription, Tt::Mlu, Tt::Text],
        ),
        (IccTag::ProfileSequenceDesc, &[Tt::ProfileSequenceDesc]),
        (IccTag::Technology, &[Tt::Signature]),
        (IccTag::ColorimetricIntentImageState, &[Tt::Signature]),
        (IccTag::PerceptualRenderingIntentGamut, &[Tt::Signature]),
        (IccTag::SaturationRenderingIntentGamut, &[Tt::Signature]),
        (IccTag::Measurement, &[Tt::Measurement]),
        (IccTag::Ps2CRD0, &[Tt::Data]),
        (IccTag::Ps2CRD1, &[Tt::Data]),
        (IccTag::Ps2CRD2, &[Tt::Data]),
        (IccTag::Ps2CRD3, &[Tt::Data]),
        (IccTag::Ps2CSA, &[Tt::Data]),
        (IccTag::Ps2RenderingIntent, &[Tt::Data]),
        (
            IccTag::ViewingCondDesc,
            &[Tt::TextDescription, Tt::Mlu, Tt::Text],
        ),
        (IccTag::UcrBg, &[Tt::UcrBg]),
        (IccTag::CrdInfo, &[Tt::CrdInfo]),
        (IccTag::DToB0, &[Tt::MultiProcessElement]),
        (IccTag::DToB1, &[Tt::MultiProcessElement]),
        (IccTag::DToB2, &[Tt::MultiProcessElement]),
        (IccTag::DToB3, &[Tt::MultiProcessElement]),
        (IccTag::BToD0, &[Tt::MultiProcessElement]),
        (IccTag::BToD1, &[Tt::MultiProcessElement]),
        (IccTag::BToD2, &[Tt::MultiProcessElement]),
        (IccTag::BToD3, &[Tt::MultiProcessElement]),
        (IccTag::ScreeningDesc, &[Tt::TextDescription]),
        (IccTag::ViewingConditions, &[Tt::ViewingConditions]),
        (IccTag::Screening, &[Tt::Screening]),
        (IccTag::Vcgt, &[Tt::Vcgt]),
        (IccTag::Meta, &[Tt::Dict]),
        (IccTag::ProfileSequenceId, &[Tt::ProfileSequenceId]),
        (IccTag::ProfileDescriptionML, &[Tt::Mlu]),
        (IccTag::ArgyllArts, &[Tt::S15Fixed16Array]),
    ];

    for (tag, types) in TAG_DATA_TYPES {
        if *tag == for_tag {
            return Some(types);
        }
    }
    None
}

type SerResult = Result<(), SerError>;

impl IccValue {
    fn serialize<T: io::Write>(&self, output: &mut T) -> SerResult {
        match self {
            IccValue::Cxyz(xyz) => xyz_ser(*xyz, output),
            IccValue::Chromaticity(r, g, b) => chromaticity_ser((*r, *g, *b), output),
            IccValue::ColorantOrder(order) => colorant_order_ser(order, output),
            IccValue::Curve(curve) => curve_ser(curve, output),
            IccValue::S15Fixed16Array(array) => s15fixed16_array_ser(array, output),
            IccValue::U16Fixed16Array(array) => u16fixed16_array_ser(array, output),
            IccValue::Signature(sig) => signature_ser(*sig, output),
            IccValue::Text(text) => text_ser(text, output),
            IccValue::Mlu(mlu) => mlu_ser(mlu, output),
            _ => todo!(),
        }
    }
}

type DeserResult = Result<IccValue, DataDeserError>;
type TypeDeserFn = fn(buf: &[u8]) -> DeserResult;

fn tag_deserializer(for_type: IccDataType) -> Option<TypeDeserFn> {
    type Tt = IccDataType;
    match for_type {
        Tt::Chromaticity => Some(chromaticity_deser),
        Tt::ColorantOrder => Some(colorant_order_deser),
        Tt::ColorantTable => None,
        Tt::CrdInfo => None,
        Tt::Curve => Some(curve_deser),
        Tt::Data => Some(data_deser),
        Tt::Dict => None,
        Tt::DateTime => None,
        Tt::DeviceSettings => None,
        Tt::Lut16 => None,
        Tt::Lut8 => None,
        Tt::LutAToB => None,
        Tt::LutBToA => None,
        Tt::Measurement => None,
        Tt::Mlu => Some(mlu_deser),
        Tt::MultiProcessElement => None,
        Tt::NamedColor => None,
        Tt::NamedColor2 => None,
        Tt::ParametricCurve => Some(parametric_curve_deser),
        Tt::ProfileSequenceDesc => None,
        Tt::ProfileSequenceId => None,
        Tt::ResponseCurveSet16 => None,
        Tt::S15Fixed16Array => Some(s15fixed16_array_deser),
        Tt::Screening => None,
        Tt::Signature => Some(signature_deser),
        Tt::Text => Some(text_deser),
        Tt::TextDescription => Some(text_description_deser),
        Tt::U16Fixed16Array => Some(u16fixed16_array_deser),
        Tt::UcrBg => None,
        Tt::UInt16Array => None,
        Tt::UInt32Array => None,
        Tt::UInt64Array => None,
        Tt::UInt8Array => None,
        Tt::Vcgt => None,
        Tt::ViewingConditions => None,
        Tt::Xyz => Some(xyz_deser),
    }
}

fn xyz_ser<T: io::Write>(xyz: Cxyz, mut buf: T) -> SerResult {
    buf.write_u32::<BE>(IccDataType::Xyz.into())?;
    buf.write_u32::<BE>(0)?;
    buf.write_i32::<BE>(s15f16::try_from(xyz.x)?.to_bytes())?;
    buf.write_i32::<BE>(s15f16::try_from(xyz.y)?.to_bytes())?;
    buf.write_i32::<BE>(s15f16::try_from(xyz.z)?.to_bytes())?;
    Ok(())
}
fn xyz_deser(buf: &[u8]) -> DeserResult {
    let mut buf = io::Cursor::new(buf);
    let x = s15f16::from_bytes(buf.read_i32::<BE>()?).into();
    let y = s15f16::from_bytes(buf.read_i32::<BE>()?).into();
    let z = s15f16::from_bytes(buf.read_i32::<BE>()?).into();
    Ok(IccValue::Cxyz(Cxyz { x, y, z }))
}
fn chromaticity_ser<T: io::Write>(colorants: (CxyY, CxyY, CxyY), mut buf: T) -> SerResult {
    buf.write_u32::<BE>(IccDataType::Chromaticity.into())?;
    buf.write_u32::<BE>(0)?;
    let channels = 3;
    buf.write_u16::<BE>(channels)?;
    buf.write_u16::<BE>(0)?; // phosphor or colorant type unknown
    buf.write_u32::<BE>(u16f16::try_from((colorants.0).x)?.to_bytes())?;
    buf.write_u32::<BE>(u16f16::try_from((colorants.0).y)?.to_bytes())?;
    buf.write_u32::<BE>(u16f16::try_from((colorants.1).x)?.to_bytes())?;
    buf.write_u32::<BE>(u16f16::try_from((colorants.1).y)?.to_bytes())?;
    buf.write_u32::<BE>(u16f16::try_from((colorants.2).x)?.to_bytes())?;
    buf.write_u32::<BE>(u16f16::try_from((colorants.2).y)?.to_bytes())?;
    Ok(())
}
fn chromaticity_deser(buf: &[u8]) -> DeserResult {
    let buf_len = buf.len();
    let mut buf = io::Cursor::new(buf);
    let mut channels = buf.read_u16::<BE>()?;

    // recovers from a bug introduced in early versions of lcms1
    if channels == 0 && buf_len == 32 {
        buf.read_u16::<BE>()?;
        channels = buf.read_u16::<BE>()?;
    }

    if channels != 3 {
        Err(DataDeserError::ChromaticityChannels(channels))?;
    }

    let _table = buf.read_u16::<BE>()?;
    let red_x = u16f16::from_bytes(buf.read_u32::<BE>()?).into();
    let red_y = u16f16::from_bytes(buf.read_u32::<BE>()?).into();
    let green_x = u16f16::from_bytes(buf.read_u32::<BE>()?).into();
    let green_y = u16f16::from_bytes(buf.read_u32::<BE>()?).into();
    let blue_x = u16f16::from_bytes(buf.read_u32::<BE>()?).into();
    let blue_y = u16f16::from_bytes(buf.read_u32::<BE>()?).into();

    Ok(IccValue::Chromaticity(
        CxyY {
            x: red_x,
            y: red_y,
            Y: 1.,
        },
        CxyY {
            x: green_x,
            y: green_y,
            Y: 1.,
        },
        CxyY {
            x: blue_x,
            y: blue_y,
            Y: 1.,
        },
    ))
}

const MAX_CHANNELS: usize = 16;

fn colorant_order_ser<T: io::Write>(order: &[u8], mut buf: T) -> SerResult {
    buf.write_u32::<BE>(IccDataType::ColorantOrder.into())?;
    buf.write_u32::<BE>(0)?;
    buf.write_u32::<BE>(order.len() as u32)?;
    buf.write_all(order)?;
    Ok(())
}
fn colorant_order_deser(buf: &[u8]) -> DeserResult {
    let mut buf = io::Cursor::new(buf);
    let count = buf.read_u32::<BE>()? as usize;
    if count > MAX_CHANNELS {
        Err(DataDeserError::ColorantOrderCount(count))?
    }

    let mut colorant_order = Vec::with_capacity(count);
    colorant_order.resize(count, 0);
    // (no end marker)

    buf.read_exact(&mut colorant_order)?;

    Ok(IccValue::ColorantOrder(colorant_order))
}

pub fn u8fixed8_to_f64(i: u16) -> f64 {
    let lsb = i & 0xFF;
    let msb = i >> 8;

    msb as f64 + (lsb as f64 / 256.)
}

fn curve_ser<T: io::Write>(curve: &ToneCurve, mut buf: T) -> SerResult {
    use crate::tone_curve::CurveType;

    // try map 1:1
    if curve.segments.len() == 1 {
        let segment = &curve.segments[0];
        match &segment.curve {
            CurveType::Const(a) => {
                buf.write_u32::<BE>(IccDataType::ParametricCurve.into())?;
                buf.write_u32::<BE>(0)?;
                buf.write_u16::<BE>(2)?;
                buf.write_u16::<BE>(0)?;
                buf.write_i32::<BE>(s15f16::try_from(0.)?.to_bytes())?;
                buf.write_i32::<BE>(s15f16::try_from(0.)?.to_bytes())?;
                buf.write_i32::<BE>(s15f16::try_from(0.)?.to_bytes())?;
                buf.write_i32::<BE>(s15f16::try_from(*a)?.to_bytes())?;
                return Ok(());
            }
            CurveType::Table(table) => {
                buf.write_u32::<BE>(IccDataType::Curve.into())?;
                buf.write_u32::<BE>(0)?;
                buf.write_u32::<BE>(table.len() as u32)?;
                for i in table {
                    buf.write_u16::<BE>(*i)?;
                }
                return Ok(());
            }
            CurveType::IccParam(curve) if !curve.is_inverted() => {
                buf.write_u32::<BE>(IccDataType::ParametricCurve.into())?;
                buf.write_u32::<BE>(0)?;
                buf.write_u16::<BE>(curve.icc_type())?;
                buf.write_u16::<BE>(0)?;
                for param in curve.params() {
                    buf.write_i32::<BE>(s15f16::try_from(param)?.to_bytes())?;
                }
                return Ok(());
            }
            _ => (),
        }
    }

    // sample curve
    let mut sample_count = 4096;

    for segment in &curve.segments {
        if let CurveType::Sampled(samples) = &segment.curve {
            sample_count = (samples.len() as u32).max(sample_count);
        }
    }

    buf.write_u32::<BE>(IccDataType::Curve.into())?;
    buf.write_u32::<BE>(0)?;
    buf.write_u32::<BE>(sample_count)?;

    for i in 0..sample_count {
        let x = i as f64 / sample_count as f64;
        let value = (curve.eval(x).unwrap_or(0.) * 65535.).max(0.).min(65535.) as u16;
        buf.write_u16::<BE>(value)?;
    }

    Ok(())
}
fn curve_deser(buf: &[u8]) -> DeserResult {
    let mut buf = io::Cursor::new(buf);

    let count = buf.read_u32::<BE>()? as usize;

    let curve = match count {
        0 => {
            // linear
            ToneCurve::new_icc_parametric(0, &[1.])
        }
        1 => {
            // gamma exponent
            let gamma = u8fixed8_to_f64(buf.read_u16::<BE>()?);

            ToneCurve::new_icc_parametric(0, &[gamma])
        }
        _ => {
            if count > 0x7FFF {
                Err(DataDeserError::InvalidToneCurve)?
            }

            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(buf.read_u16::<BE>()?);
            }

            Some(ToneCurve::new_table(values))
        }
    };

    match curve {
        Some(curve) => Ok(IccValue::Curve(curve)),
        None => Err(DataDeserError::InvalidToneCurve)?,
    }
}

fn s15fixed16_array_ser<T: io::Write>(array: &[s15f16], mut buf: T) -> SerResult {
    buf.write_u32::<BE>(IccDataType::S15Fixed16Array.into())?;
    buf.write_u32::<BE>(0)?;
    for i in array {
        buf.write_i32::<BE>(i.to_bytes())?;
    }
    Ok(())
}
fn s15fixed16_array_deser(buf: &[u8]) -> DeserResult {
    let buf_len = buf.len();
    let mut buf = io::Cursor::new(buf);
    let count = buf_len / mem::size_of::<s15f16>();

    let mut values = Vec::with_capacity(count);

    for _ in 0..count {
        values.push(s15f16::from_bytes(buf.read_i32::<BE>()?));
    }

    Ok(IccValue::S15Fixed16Array(values))
}

fn u16fixed16_array_ser<T: io::Write>(array: &[u16f16], mut buf: T) -> SerResult {
    buf.write_u32::<BE>(IccDataType::U16Fixed16Array.into())?;
    buf.write_u32::<BE>(0)?;
    for i in array {
        buf.write_u32::<BE>(i.to_bytes())?;
    }
    Ok(())
}
fn u16fixed16_array_deser(buf: &[u8]) -> DeserResult {
    let buf_len = buf.len();
    let mut buf = io::Cursor::new(buf);
    let count = buf_len / mem::size_of::<u16f16>();

    let mut values = Vec::with_capacity(count);

    for _ in 0..count {
        values.push(u16f16::from_bytes(buf.read_u32::<BE>()?));
    }

    Ok(IccValue::U16Fixed16Array(values))
}

fn signature_ser<T: io::Write>(signature: u32, mut buf: T) -> SerResult {
    buf.write_u32::<BE>(signature)?;
    Ok(())
}
fn signature_deser(buf: &[u8]) -> DeserResult {
    if buf.len() < 4 {
        Err(io::Error::from(io::ErrorKind::UnexpectedEof))?
    }
    Ok(IccValue::Signature(BE::read_u32(buf)))
}

/// For some reason, bits of text often have a lot of trailing null bytes, so here’s a function that
/// returns a sub-slice with everything up to and including the first.
fn cstr_slice(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|x| *x == 0) {
        Some(i) => &buf[..i + 1],
        None => buf,
    }
}

fn text_ser<T: io::Write>(text: &str, mut buf: T) -> SerResult {
    buf.write_u32::<BE>(IccDataType::Text.into())?;
    buf.write_u32::<BE>(0)?;

    for c in text.chars() {
        if c.is_ascii() {
            buf.write_u8(c as u8)?;
        } else {
            buf.write_u8('?' as u8)?;
        }
    }

    Ok(())
}
fn text_deser(buf: &[u8]) -> DeserResult {
    use std::ffi::CStr;

    let cstr_buf = cstr_slice(&buf);
    let cstr = CStr::from_bytes_with_nul(cstr_buf)
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
    Ok(IccValue::Text(cstr.to_string_lossy().to_string()))
}

fn data_deser(buf: &[u8]) -> DeserResult {
    let data_len = buf.len() - 1;
    let mut buf = io::Cursor::new(buf);
    let mut data = Vec::with_capacity(data_len);
    let flags = buf.read_u32::<BE>()?;
    buf.read_exact(&mut data)?;

    Ok(IccValue::Data { flags, data })
}

fn text_description_deser(buf: &[u8]) -> DeserResult {
    use std::ffi::CStr;

    let mut buf = io::Cursor::new(buf);

    let ascii_count = buf.read_u32::<BE>()? as usize;
    let mut cstr_buf = Vec::with_capacity(ascii_count);
    cstr_buf.resize(ascii_count, 0);
    buf.read_exact(&mut cstr_buf)?;
    let cstr = CStr::from_bytes_with_nul(cstr_slice(&cstr_buf))
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

    // ignore unicode and scriptcode
    Ok(IccValue::Text(cstr.to_string_lossy().to_string()))
}

fn parametric_curve_deser(buf: &[u8]) -> DeserResult {
    const PARAM_COUNT_BY_TYPE: &[u8] = &[1, 3, 4, 5, 7];

    let mut buf = io::Cursor::new(buf);

    let p_type = buf.read_u16::<BE>()?;
    let _reserved = buf.read_u16::<BE>()?;

    if p_type > 4 {
        // unknown
        Err(DataDeserError::UnknownParametricCurve(p_type))?
    }

    let mut params = Vec::new();
    for _ in 0..PARAM_COUNT_BY_TYPE[p_type as usize] {
        params.push(s15f16::from_bytes(buf.read_i32::<BE>()?).into());
    }

    match ToneCurve::new_icc_parametric(p_type, &params) {
        Some(curve) => Ok(IccValue::Curve(curve)),
        None => Err(DataDeserError::InvalidToneCurve)?,
    }
}

fn mlu_ser<T: io::Write>(mlu: &Mlu, mut buf: T) -> SerResult {
    buf.write_u32::<BE>(IccDataType::Mlu.into())?;
    buf.write_u32::<BE>(0)?;

    buf.write_u32::<BE>(mlu.entries.len() as u32)?;
    buf.write_u32::<BE>(12)?;

    let mut string_storage_offset = 16 + mlu.entries.len() as u32 * 12;
    for ((lang, country), string) in &mlu.entries {
        buf.write_u16::<BE>(*lang)?;
        buf.write_u16::<BE>(*country)?;
        let utf16_len = string.encode_utf16().count() as u32;
        let byte_len = utf16_len * 2;
        buf.write_u32::<BE>(byte_len)?;
        buf.write_u32::<BE>(string_storage_offset)?;
        string_storage_offset += byte_len;
    }

    for (_, string) in &mlu.entries {
        for i in string.encode_utf16() {
            buf.write_u16::<BE>(i)?;
        }
    }

    Ok(())
}

// > Do NOT trust SizeOfTag as there is an issue on the definition of profileSequenceDescTag. See
// > the TechNote from Max Derhak and Rohit Patil about this: basically the size of the string table
// > should be guessed and cannot be taken from the size of tag if this tag is embedded as part of
// > bigger structures (profileSequenceDescTag, for instance)
fn mlu_deser(buf: &[u8]) -> DeserResult {
    let buf_len = buf.len();
    let mut buf = io::Cursor::new(buf);

    let count = buf.read_u32::<BE>()? as usize;
    let record_len = buf.read_u32::<BE>()?;

    if record_len != 12 {
        // not supported
        Err(DataDeserError::InvalidMluRecordSize(record_len))?
    }

    let mut mlu = Mlu::new();

    for _ in 0..count {
        let lang = buf.read_u16::<BE>()?;
        let country = buf.read_u16::<BE>()?;
        let len = buf.read_u32::<BE>()? as usize / 2;
        let offset = buf.read_u32::<BE>()? as usize;

        if offset + len > buf_len + 8 {
            Err(DataDeserError::InvalidMlu)?
        }

        let pos = buf.position();

        buf.seek(io::SeekFrom::Start(offset as u64 - 8))?;

        let mut str_buf = Vec::with_capacity(len);
        for _ in 0..len {
            match buf.read_u16::<BE>()? {
                0 => break,
                c => str_buf.push(c),
            }
        }

        let string = String::from_utf16_lossy(&str_buf);
        mlu.insert_raw(lang, country, string);

        buf.set_position(pos);
    }

    Ok(IccValue::Mlu(mlu))
}
