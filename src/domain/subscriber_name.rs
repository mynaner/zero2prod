/*
 * @Date: 2025-07-14 23:26:53
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-14 23:35:05
 * @FilePath: /zero2prod/src/domain/subscriber_name.rs
 */
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubScriberName(String);

impl SubScriberName {
    pub fn parse(s: String) -> Result<Self, String> {
        // trim 移除开始结尾的空字符
        // is_empty 是否为空
        let is_enpty_or_whitespace = s.trim().is_empty();

        // 字符是否太长
        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '>', '<', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_enpty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("{} is not a valid subscriber name,", s))
        } else {
            Ok(Self(s))
        }
    }

    // 读取值的机会,没有权利更改
    // pub fn inner(self) -> String {
    //     self.0
    // }
    // 这样跟  pub struct SbuScriberName(pub String); 的意图一样了
    // pub fn inner_mut(&mut self) -> &mut str {
    //     &mut self.0
    // }

    // pub fn inner_ref(&self) -> &str {
    //     &self.0
    // }
}

impl AsRef<str> for SubScriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};

    use crate::domain::SubScriberName;
    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(SubScriberName::parse(name));
    }
    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubScriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubScriberName::parse(name));
    }
    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in ['/', '(', ')', '"', '>', '<', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubScriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubScriberName::parse(name));
    }
}
