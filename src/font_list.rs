use std::collections::HashMap;
use ucd::{Codepoint, UnicodeBlock};

fn unicode_plane(codepoint: char) -> u32 {
    (codepoint as u32) >> 16
}

// refer: https://github.com/servo/servo/blob/3f7697690aabd2d8c31bc880fcae21250244219a/components/gfx/platform/macos/font_list.rs#L38-L193
pub fn fallback_font_families(codepoint: Option<char>) -> Vec<String> {
    let mut families = vec!["Lucida Grande".to_string()];

    if let Some(codepoint) = codepoint {
        match unicode_plane(codepoint) {
            // https://en.wikipedia.org/wiki/Plane_(Unicode)#Basic_Multilingual_Plane
            0 => {
                if let Some(block) = codepoint.block() {
                    match block {
                        // UnicodeBlock::GeneralPunctuation |
                        // UnicodeBlock::SuperscriptsandSubscripts |
                        // UnicodeBlock::CurrencySymbols |
                        // UnicodeBlock::CombiningDiacriticalMarksforSymbols |
                        // UnicodeBlock::LetterlikeSymbols |
                        // UnicodeBlock::NumberForms |
                        // UnicodeBlock::Arrows |
                        // UnicodeBlock::MathematicalOperators |
                        // UnicodeBlock::MiscellaneousTechnical |
                        // UnicodeBlock::ControlPictures |
                        // UnicodeBlock::OpticalCharacterRecognition |
                        // UnicodeBlock::EnclosedAlphanumerics |
                        // UnicodeBlock::BoxDrawing |
                        // UnicodeBlock::BlockElements |
                        // UnicodeBlock::GeometricShapes |
                        // UnicodeBlock::MiscellaneousSymbols |
                        // UnicodeBlock::Dingbats |
                        // UnicodeBlock::MiscellaneousMathematicalSymbolsA |
                        // UnicodeBlock::SupplementalArrowsA |
                        // UnicodeBlock::SupplementalArrowsB |
                        // UnicodeBlock::MiscellaneousMathematicalSymbolsB |
                        // UnicodeBlock::SupplementalMathematicalOperators |
                        // UnicodeBlock::MiscellaneousSymbolsandArrows |
                        // UnicodeBlock::SupplementalPunctuation => {
                        //     families.push("Hiragino Kaku Gothic ProN");
                        //     families.push("Apple Symbols");
                        //     families.push("Menlo");
                        //     families.push("STIXGeneral");
                        // },
                        UnicodeBlock::Kanbun
                        | UnicodeBlock::Hiragana
                        | UnicodeBlock::Katakana
                        | UnicodeBlock::CJKStrokes
                        | UnicodeBlock::KatakanaPhoneticExtensions => {
                            families.push("Hiragino Sans GB".into());
                        }

                        // UnicodeBlock::YijingHexagramSymbols |
                        // UnicodeBlock::CyrillicExtendedB |
                        // UnicodeBlock::Bamum |
                        // UnicodeBlock::ModifierToneLetters |
                        // UnicodeBlock::LatinExtendedD |
                        // UnicodeBlock::ArabicPresentationFormsA |
                        // UnicodeBlock::HalfwidthandFullwidthForms |
                        // UnicodeBlock::Specials => "Apple Symbols".into(),
                        _ => {}
                    }
                }
            }

            // https://en.wikipedia.org/wiki/Plane_(Unicode)#Supplementary_Multilingual_Plane
            // 1 => {
            //     families.push("Apple Symbols");
            //     families.push("STIXGeneral");
            // },

            // https://en.wikipedia.org/wiki/Plane_(Unicode)#Supplementary_Ideographic_Plane
            // 2 => {
            //     // Systems with MS Office may have these fonts
            //     families.push("MingLiU-ExtB");
            //     families.push("SimSun-ExtB");
            // },
            _ => {}
        };
    }

    families.push("Geneva".into());
    families
}

pub fn get_generic_fonts() -> HashMap<String, String> {
    fn append(generic_fonts: &mut HashMap<String, String>, key: &str, val: &str) {
        generic_fonts.insert(key.to_string(), val.to_string());
    }

    let mut generic_fonts = HashMap::with_capacity(5);
    append(&mut generic_fonts, "serif", "Times New Roman");
    append(&mut generic_fonts, "sans-serif", "Helvetica");
    append(&mut generic_fonts, "cursive", "Apple Chancery");
    append(&mut generic_fonts, "fantasy", "Papyrus");
    append(&mut generic_fonts, "monospace", "Menlo");

    generic_fonts
}

pub static DEFAULT_FONT_FAMILY_NAME: &'static str = "Times New Roman";
