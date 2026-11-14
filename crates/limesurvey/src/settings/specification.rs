use super::SettingValue;

pub(super) enum Kind {
    Flag,
    Integer,
    Code(&'static [&'static str]),
}

pub(super) struct Specification {
    pub name: &'static str,
    pub kind: Kind,
}

impl Specification {
    pub fn parse(&self, value: &str) -> Option<SettingValue> {
        match self.kind {
            Kind::Flag => match value {
                "Y" => Some(SettingValue::Flag(true)),
                "N" => Some(SettingValue::Flag(false)),
                _ => None,
            },
            Kind::Integer => value
                .parse::<u32>()
                .ok()
                .filter(|_| !value.starts_with('+'))
                .map(SettingValue::Integer),
            Kind::Code(values) => values
                .iter()
                .find(|&&code| code == value)
                .map(|&code| SettingValue::Code(code)),
        }
    }
}

macro_rules! flag {
    ($name:literal) => {
        Specification {
            name: $name,
            kind: Kind::Flag,
        }
    };
}

pub(super) const SETTINGS: &[Specification] = &[
    flag!("anonymized"),
    flag!("savetimings"),
    flag!("datestamp"),
    flag!("usecookie"),
    flag!("allowregister"),
    flag!("allowsave"),
    flag!("autoredirect"),
    flag!("allowprev"),
    flag!("printanswers"),
    flag!("ipaddr"),
    flag!("ipanonymize"),
    flag!("refurl"),
    flag!("publicstatistics"),
    flag!("publicgraphs"),
    flag!("listpublic"),
    flag!("htmlemail"),
    flag!("sendconfirmation"),
    flag!("tokenanswerspersistence"),
    flag!("assessments"),
    flag!("showxquestions"),
    flag!("shownoanswer"),
    flag!("showwelcome"),
    flag!("showprogress"),
    flag!("nokeyboard"),
    flag!("alloweditaftercompletion"),
    Specification {
        name: "format",
        kind: Kind::Code(&["A", "G", "S"]),
    },
    Specification {
        name: "showgroupinfo",
        kind: Kind::Code(&["B", "D", "N", "X"]),
    },
    Specification {
        name: "showqnumcode",
        kind: Kind::Code(&["B", "C", "N", "X"]),
    },
    Specification {
        name: "questionindex",
        kind: Kind::Code(&["0", "1", "2"]),
    },
    Specification {
        name: "navigationdelay",
        kind: Kind::Integer,
    },
];
