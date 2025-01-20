use std::fmt;

enum_primitive! {
    /// Rendering intents.
    ///
    /// When in doubt, use perceptual. Refer to the ICC specification for details.
    pub Intent (u32) {
        Perceptual = 0,
        RelativeColorimetric = 1,
        Saturation = 2,
        AbsoluteColorimetric = 3,
    }
}

enum_primitive! {
    /// ICC tags.
    pub IccTag (u32) {
        /// `A2B0`
        AToB0 = 0x41324230,
        /// `A2B1`
        AToB1 = 0x41324231,
        /// `A2B2`
        AToB2 = 0x41324232,
        /// `bXYZ`
        BlueColorant = 0x6258595A,
        /// `bTRC`
        BlueTRC = 0x62545243,
        /// `B2A0`
        BToA0 = 0x42324130,
        /// `B2A1`
        BToA1 = 0x42324131,
        /// `B2A2`
        BToA2 = 0x42324132,
        /// `calt`
        CalibrationDateTime = 0x63616C74,
        /// `targ`
        CharTarget = 0x74617267,
        /// `chad`
        ChromaticAdaptation = 0x63686164,
        /// `chrm`
        Chromaticity = 0x6368726D,
        /// `clro`
        ColorantOrder = 0x636C726F,
        /// `clrt`
        ColorantTable = 0x636C7274,
        /// `clot`
        ColorantTableOut = 0x636C6F74,
        /// `ciis`
        ColorimetricIntentImageState = 0x63696973,
        /// `cprt`
        Copyright = 0x63707274,
        /// `crdi`
        CrdInfo = 0x63726469,
        /// `data`
        Data = 0x64617461,
        /// `dtim`
        DateTime = 0x6474696D,
        /// `dmnd`
        DeviceMfgDesc = 0x646D6E64,
        /// `dmdd`
        DeviceModelDesc = 0x646D6464,
        /// `devs`
        DeviceSettings = 0x64657673,
        /// `D2B0`
        DToB0 = 0x44324230,
        /// `D2B1`
        DToB1 = 0x44324231,
        /// `D2B2`
        DToB2 = 0x44324232,
        /// `D2B3`
        DToB3 = 0x44324233,
        /// `B2D0`
        BToD0 = 0x42324430,
        /// `B2D1`
        BToD1 = 0x42324431,
        /// `B2D2`
        BToD2 = 0x42324432,
        /// `B2D3`
        BToD3 = 0x42324433,
        /// `gamt`
        Gamut = 0x67616D74,
        /// `kTRC`
        GrayTRC = 0x6b545243,
        /// `gXYZ`
        GreenColorant = 0x6758595A,
        /// `gTRC`
        GreenTRC = 0x67545243,
        /// `lumi`
        Luminance = 0x6C756d69,
        /// `meas`
        Measurement = 0x6D656173,
        /// `bkpt`
        MediaBlackPoint = 0x626B7074,
        /// `wtpt`
        MediaWhitePoint = 0x77747074,
        /// `ncol` (deprecated)
        NamedColor = 0x6E636f6C,
        /// `ncl2`
        NamedColor2 = 0x6E636C32,
        /// `resp`
        OutputResponse = 0x72657370,
        /// `rig0`
        PerceptualRenderingIntentGamut = 0x72696730,
        /// `pre0`
        Preview0 = 0x70726530,
        /// `pre1`
        Preview1 = 0x70726531,
        /// `pre2`
        Preview2 = 0x70726532,
        /// `desc`
        ProfileDescription = 0x64657363,
        /// `dscm`
        ProfileDescriptionML = 0x6473636d,
        /// `pseq`
        ProfileSequenceDesc = 0x70736571,
        /// `psid`
        ProfileSequenceId = 0x70736964,
        /// `psd0`
        Ps2CRD0 = 0x70736430,
        /// `psd1`
        Ps2CRD1 = 0x70736431,
        /// `psd2`
        Ps2CRD2 = 0x70736432,
        /// `psd3`
        Ps2CRD3 = 0x70736433,
        /// `ps2s`
        Ps2CSA = 0x70733273,
        /// `ps2i`
        Ps2RenderingIntent = 0x70733269,
        /// `rXYZ`
        RedColorant = 0x7258595A,
        /// `rTRC`
        RedTRC = 0x72545243,
        /// `rig2`
        SaturationRenderingIntentGamut = 0x72696732,
        /// `scrd`
        ScreeningDesc = 0x73637264,
        /// `scrn`
        Screening = 0x7363726E,
        /// `tech`
        Technology = 0x74656368,
        /// `bfd `
        UcrBg = 0x62666420,
        /// `vued`
        ViewingCondDesc = 0x76756564,
        /// `view`
        ViewingConditions = 0x76696577,
        /// `vcgt`
        Vcgt = 0x76636774,
        /// `meta`
        Meta = 0x6D657461,
        /// `arts`
        ArgyllArts = 0x61727473,
    }
}

enum_primitive! {
    /// ICC type definitions.
    pub IccDataType (u32) {
        /// `chrm`
        Chromaticity = 0x6368726D,
        /// `clro`
        ColorantOrder = 0x636C726F,
        /// `clrt`
        ColorantTable = 0x636C7274,
        /// `crdi`
        CrdInfo = 0x63726469,
        /// `curv`
        Curve = 0x63757276,
        /// `data`
        Data = 0x64617461,
        /// `dict`
        Dict = 0x64696374,
        /// `dtim`
        DateTime = 0x6474696D,
        /// `devs`
        DeviceSettings = 0x64657673,
        /// `mft2`
        Lut16 = 0x6d667432,
        /// `mft1`
        Lut8 = 0x6d667431,
        /// `mAB `
        LutAToB = 0x6d414220,
        /// `mBA `
        LutBToA = 0x6d424120,
        /// `meas`
        Measurement = 0x6D656173,
        /// `mluc`
        Mlu = 0x6D6C7563,
        /// `mpet`
        MultiProcessElement = 0x6D706574,
        /// `ncol` (deprecated)
        NamedColor = 0x6E636f6C,
        /// `ncl2`
        NamedColor2 = 0x6E636C32,
        /// `para`
        ParametricCurve = 0x70617261,
        /// `pseq`
        ProfileSequenceDesc = 0x70736571,
        /// `psid`
        ProfileSequenceId = 0x70736964,
        /// `rcs2`
        ResponseCurveSet16 = 0x72637332,
        /// `sf32`
        S15Fixed16Array = 0x73663332,
        /// `scrn`
        Screening = 0x7363726E,
        /// `sig `
        Signature = 0x73696720,
        /// `text`
        Text = 0x74657874,
        /// `desc`
        TextDescription = 0x64657363,
        /// `uf32`
        U16Fixed16Array = 0x75663332,
        /// `bfd `
        UcrBg = 0x62666420,
        /// `ui16`
        UInt16Array = 0x75693136,
        /// `ui32`
        UInt32Array = 0x75693332,
        /// `ui64`
        UInt64Array = 0x75693634,
        /// `ui08`
        UInt8Array = 0x75693038,
        /// `vcgt`
        Vcgt = 0x76636774,
        /// `view`
        ViewingConditions = 0x76696577,
        /// `XYZ `
        Xyz = 0x58595A20,
    }
}

enum_primitive! {
    /// ICC profile classes.
    pub ProfileClass (u32) {
        /// `scnr`
        Input = 0x73636E72,
        /// `mntr`
        Display = 0x6D6E7472,
        /// `prtr`
        Output = 0x70727472,
        /// `link`
        Link = 0x6C696E6B,
        /// `abst`
        Abstract = 0x61627374,
        /// `spac`
        ColorSpace = 0x73706163,
        /// `nmcl`
        NamedColor = 0x6e6d636c,
    }
}

enum_primitive! {
    /// Values for the ICC technology tag.
    pub Technology (u32) {
        /// `dcam`
        DigitalCamera = 0x6463616D,
        /// `fscn`
        FilmScanner = 0x6673636E,
        /// `rscn`
        ReflectiveScanner = 0x7273636E,
        /// `ijet`
        InkJetPrinter = 0x696A6574,
        /// `twax`
        ThermalWaxPrinter = 0x74776178,
        /// `epho`
        ElectrophotographicPrinter = 0x6570686F,
        /// `esta`
        ElectrostaticPrinter = 0x65737461,
        /// `dsub`
        DyeSublimationPrinter = 0x64737562,
        /// `rpho`
        PhotographicPaperPrinter = 0x7270686F,
        /// `fprn`
        FilmWriter = 0x6670726E,
        /// `vidm`
        VideoMonitor = 0x7669646D,
        /// `vidc`
        VideoCamera = 0x76696463,
        /// `pjtv`
        ProjectionTelevision = 0x706A7476,
        /// `CRT `
        CRTDisplay = 0x43525420,
        /// `PMD `
        PMDisplay = 0x504D4420,
        /// `AMD `
        AMDisplay = 0x414D4420,
        /// `KPCD`
        PhotoCD = 0x4B504344,
        /// `imgs`
        PhotoImageSetter = 0x696D6773,
        /// `grav`
        Gravure = 0x67726176,
        /// `offs`
        OffsetLithography = 0x6F666673,
        /// `silk`
        Silkscreen = 0x73696C6B,
        /// `flex`
        Flexography = 0x666C6578,
        /// `mpfs`
        MotionPictureFilmScanner = 0x6D706673,
        /// `mpfr`
        MotionPictureFilmRecorder = 0x6D706672,
        /// `dmpc`
        DigitalMotionPictureCamera = 0x646D7063,
        /// `dcpj`
        DigitalCinemaProjector = 0x64636A70,
    }
}

enum_primitive! {
    /// ICC color spaces.
    pub ColorSpace (u32) {
        /// `XYZ `
        XYZ = 0x58595a20,
        /// `Lab `
        Lab = 0x4C616220,
        /// `Luv `
        Luv = 0x4C757620,
        /// `YCbr`
        YCbCr = 0x59436272,
        /// `Yxy `
        Yxy = 0x59787920,
        /// `RGB `
        RGB = 0x52474220,
        /// `GRAY`
        Gray = 0x47524159,
        /// `HSV `
        HSV = 0x48535620,
        /// `HLS `
        HLS = 0x484C5320,
        /// `CMYK`
        CMYK = 0x434D594B,
        /// `CMY `
        CMY = 0x434D5920,
        /// `MCH1`
        MCH1 = 0x4D434831,
        /// `MCH2`
        MCH2 = 0x4D434832,
        /// `MCH3`
        MCH3 = 0x4D434833,
        /// `MCH4`
        MCH4 = 0x4D434834,
        /// `MCH5`
        MCH5 = 0x4D434835,
        /// `MCH6`
        MCH6 = 0x4D434836,
        /// `MCH7`
        MCH7 = 0x4D434837,
        /// `MCH8`
        MCH8 = 0x4D434838,
        /// `MCH9`
        MCH9 = 0x4D434839,
        /// `MCHA`
        MCHA = 0x4D434841,
        /// `MCHB`
        MCHB = 0x4D434842,
        /// `MCHC`
        MCHC = 0x4D434843,
        /// `MCHD`
        MCHD = 0x4D434844,
        /// `MCHE`
        MCHE = 0x4D434845,
        /// `MCHF`
        MCHF = 0x4D434846,
        /// `nmcl`
        Named = 0x6E6D636C,
        /// `1CLR`
        S1Color = 0x31434C52,
        /// `2CLR`
        S2Color = 0x32434C52,
        /// `3CLR`
        S3Color = 0x33434C52,
        /// `4CLR`
        S4Color = 0x34434C52,
        /// `5CLR`
        S5Color = 0x35434C52,
        /// `6CLR`
        S6Color = 0x36434C52,
        /// `7CLR`
        S7Color = 0x37434C52,
        /// `8CLR`
        S8Color = 0x38434C52,
        /// `9CLR`
        S9Color = 0x39434C52,
        /// `ACLR`
        S10Color = 0x41434C52,
        /// `BCLR`
        S11Color = 0x42434C52,
        /// `CCLR`
        S12Color = 0x43434C52,
        /// `DCLR`
        S13Color = 0x44434C52,
        /// `ECLR`
        S14Color = 0x45434C52,
        /// `FCLR`
        S15Color = 0x46434C52,
        /// `LuvK`
        LuvK = 0x4C75764B,
    }
}

impl ColorSpace {
    /// Returns the number of channels in the color space.
    pub fn channels(self) -> usize {
        use ColorSpace::*;
        match self {
            MCH1 | S1Color | Gray => 1,
            MCH2 | S2Color => 2,
            XYZ | Lab | Luv | YCbCr | Yxy | RGB | HSV | HLS | CMY | MCH3 | S3Color => 3,
            LuvK | CMYK | MCH4 | S4Color => 4,
            MCH5 | S5Color => 5,
            MCH6 | S6Color => 6,
            MCH7 | S7Color => 7,
            MCH8 | S8Color => 8,
            MCH9 | S9Color => 9,
            MCHA | S10Color => 10,
            MCHB | S11Color => 11,
            MCHC | S12Color => 12,
            MCHD | S13Color => 13,
            MCHE | S14Color => 14,
            MCHF | S15Color => 15,
            Named => 0, // ?
        }
    }
}

impl fmt::Display for ColorSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            ColorSpace::XYZ => "XYZ",
            ColorSpace::Lab => "L*a*b*",
            ColorSpace::Luv => "L*u*v*",
            ColorSpace::YCbCr => "YCbCr",
            ColorSpace::Yxy => "xyY",
            ColorSpace::RGB => "RGB",
            ColorSpace::Gray => "Gray",
            ColorSpace::HSV => "HSV",
            ColorSpace::HLS => "HLS",
            ColorSpace::CMYK => "CMYK",
            ColorSpace::CMY => "CMY",
            ColorSpace::MCH1 => "multi-channel (1)",
            ColorSpace::MCH2 => "multi-channel (2)",
            ColorSpace::MCH3 => "multi-channel (3)",
            ColorSpace::MCH4 => "multi-channel (4)",
            ColorSpace::MCH5 => "multi-channel (5)",
            ColorSpace::MCH6 => "multi-channel (6)",
            ColorSpace::MCH7 => "multi-channel (7)",
            ColorSpace::MCH8 => "multi-channel (8)",
            ColorSpace::MCH9 => "multi-channel (9)",
            ColorSpace::MCHA => "multi-channel (10)",
            ColorSpace::MCHB => "multi-channel (11)",
            ColorSpace::MCHC => "multi-channel (12)",
            ColorSpace::MCHD => "multi-channel (13)",
            ColorSpace::MCHE => "multi-channel (14)",
            ColorSpace::MCHF => "multi-channel (15)",
            ColorSpace::Named => "named color",
            ColorSpace::S1Color => "1 color",
            ColorSpace::S2Color => "2 color",
            ColorSpace::S3Color => "3 color",
            ColorSpace::S4Color => "4 color",
            ColorSpace::S5Color => "5 color",
            ColorSpace::S6Color => "6 color",
            ColorSpace::S7Color => "7 color",
            ColorSpace::S8Color => "8 color",
            ColorSpace::S9Color => "9 color",
            ColorSpace::S10Color => "10 color",
            ColorSpace::S11Color => "11 color",
            ColorSpace::S12Color => "12 color",
            ColorSpace::S13Color => "13 color",
            ColorSpace::S14Color => "14 color",
            ColorSpace::S15Color => "15 color",
            ColorSpace::LuvK => "L*u*v*K",
        };
        write!(f, "{}", name)
    }
}
