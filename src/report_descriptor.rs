use anyhow::Result;

pub struct RawReport(pub(crate) Vec<u8>);

impl RawReport {
    pub fn data(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug)]
enum ReportItemFormat {
    Short,
    Long,
}

#[derive(Debug, Copy, Clone)]
enum ReportItemType {
    Main = 0,
    Global = 1,
    Local = 2,
    Reserved = 3,
}

impl ReportItemType {
    fn parse(val: u8) -> Self {
        match val {
            0 => ReportItemType::Main,
            1 => ReportItemType::Global,
            2 => ReportItemType::Local,
            3 => ReportItemType::Reserved,
            _ => panic!("Parsing a non 2-bit value for Report type"),
        }
    }
}

#[derive(Debug)]
pub struct ReportItem<'a> {
    typ: ReportItemType,
    tag: Tag,
    format: ReportItemFormat,
    data: &'a [u8],
}

impl<'a> ReportItem<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, usize)> {
        // first byte defines size, type and tag/format

        let first_byte = data[0];

        let size = first_byte & 0x3;

        let raw_type = (data[0] >> 2) & 0x3;

        let typ = ReportItemType::parse(raw_type);

        let tag = Tag::parse((data[0] >> 4) & 0xf, typ);

        // Check if it is a long item
        if tag == Tag::LongItem {
            let size = data[1];
            let tag = Tag::parse(data[2], typ);

            Ok((
                ReportItem {
                    typ,
                    tag,
                    format: ReportItemFormat::Long,
                    data: &data[3..3 + (size as usize)],
                },
                (size + 3) as usize,
            ))
        } else {
            Ok((
                ReportItem {
                    typ,
                    tag,
                    format: ReportItemFormat::Short,
                    data: &data[1..1 + (size as usize)],
                },
                (size + 1) as usize,
            ))
        }
    }
}

#[derive(Debug, PartialEq)]
enum Tag {
    Global(GlobalTag),
    Main(MainTag),
    Local(LocalTag),
    LongItem,
    Unknown(u8),
}

impl Tag {
    fn parse(val: u8, format: ReportItemType) -> Self {
        if val == 0b1111 {
            return Tag::LongItem;
        }

        match format {
            ReportItemType::Global => Tag::Global(val.into()),
            ReportItemType::Main => Tag::Main(val.into()),
            ReportItemType::Local => Tag::Local(val.into()),
            _ => Tag::Unknown(val),
        }
    }
}

#[derive(Debug, PartialEq)]
enum GlobalTag {
    UsagePage,
    LogicalMinimum,
    LogicalMaximum,
    PhysicalMinimum,
    PhysicalMaximum,
    UnitExponent,
    Unit,
    ReportSize,
    ReportId,
    ReportCount,
    Push,
    Pop,
    Reserved(u8),
}

impl From<u8> for GlobalTag {
    fn from(val: u8) -> Self {
        use GlobalTag::*;
        match val {
            0 => UsagePage,
            1 => LogicalMinimum,
            2 => LogicalMaximum,
            3 => PhysicalMinimum,
            4 => PhysicalMaximum,
            5 => UnitExponent,
            6 => Unit,
            7 => ReportSize,
            8 => ReportId,
            9 => ReportCount,
            10 => Push,
            11 => Pop,
            other => Reserved(other),
        }
    }
}

#[derive(Debug, PartialEq)]
enum MainTag {
    Input,
    Output,
    Feature,
    Collection,
    EndCollection,
    Reserved(u8),
}

impl From<u8> for MainTag {
    fn from(val: u8) -> Self {
        use MainTag::*;
        match val {
            0b1000 => Input,
            0b1001 => Output,
            0b1011 => Feature,
            0b1010 => Collection,
            0b1100 => EndCollection,
            other => Reserved(other),
        }
    }
}

#[derive(Debug, PartialEq)]
enum LocalTag {
    Usage,
    UsageMinimum,
    UsageMaximum,
    DesignatorIndex,
    DesignatorMinimum,
    DesignatorMaximum,
    StringIndex,
    StringMinimum,
    StringMaximum,
    Delimiter,
    Reserved(u8),
}

impl From<u8> for LocalTag {
    fn from(val: u8) -> Self {
        use LocalTag::*;
        match val {
            0b0000 => Usage,
            0b0001 => UsageMinimum,
            0b0010 => UsageMaximum,
            0b0011 => DesignatorIndex,
            0b0100 => DesignatorMinimum,
            0b0101 => DesignatorMaximum,
            0b0111 => StringIndex,
            0b1000 => StringMinimum,
            0b1001 => StringMaximum,
            0b1010 => Delimiter,
            other => Reserved(other),
        }
    }
}
