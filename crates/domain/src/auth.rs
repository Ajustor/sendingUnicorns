//! How a request proves who sends it.

/// Where an API key goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyLocation {
    #[default]
    Header,
    Query,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Auth {
    /// The folder's, else the collection's.
    #[default]
    Inherit,
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        location: KeyLocation,
    },
}

/// What an auth adds to a request, once its variables are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthEffect {
    Header(String, String),
    Query(String, String),
}

impl Auth {
    pub const KINDS: [&'static str; 5] = [
        "Inherit",
        "No auth",
        "Bearer token",
        "Basic auth",
        "API key",
    ];

    /// Index in [`Self::KINDS`].
    pub fn kind_index(&self) -> usize {
        match self {
            Self::Inherit => 0,
            Self::None => 1,
            Self::Bearer { .. } => 2,
            Self::Basic { .. } => 3,
            Self::ApiKey { .. } => 4,
        }
    }

    /// An empty auth of the kind at `index` of [`Self::KINDS`].
    pub fn of_kind(index: usize) -> Self {
        match index {
            1 => Self::None,
            2 => Self::Bearer {
                token: String::new(),
            },
            3 => Self::Basic {
                username: String::new(),
                password: String::new(),
            },
            4 => Self::ApiKey {
                key: String::new(),
                value: String::new(),
                location: KeyLocation::Header,
            },
            _ => Self::Inherit,
        }
    }

    /// The header or query param to add; `render` resolves variables.
    pub fn effect(&self, render: &mut impl FnMut(&str) -> String) -> Option<AuthEffect> {
        match self {
            Self::Bearer { token } => Some(AuthEffect::Header(
                "Authorization".into(),
                format!("Bearer {}", render(token)),
            )),
            Self::Basic { username, password } => {
                let credentials = format!("{}:{}", render(username), render(password));
                Some(AuthEffect::Header(
                    "Authorization".into(),
                    format!("Basic {}", base64(credentials.as_bytes())),
                ))
            }
            Self::ApiKey {
                key,
                value,
                location,
            } if !key.trim().is_empty() => {
                let (key, value) = (render(key.trim()), render(value));
                Some(match location {
                    KeyLocation::Header => AuthEffect::Header(key, value),
                    KeyLocation::Query => AuthEffect::Query(key, value),
                })
            }
            Self::Inherit | Self::None | Self::ApiKey { .. } => None,
        }
    }
}

/// Standard base64, with padding.
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected);
        }
    }

    #[test]
    fn effects() {
        let mut render = |s: &str| s.replace("{{t}}", "abc");
        assert_eq!(
            Auth::Bearer {
                token: "{{t}}".into()
            }
            .effect(&mut render),
            Some(AuthEffect::Header(
                "Authorization".into(),
                "Bearer abc".into()
            ))
        );
        assert_eq!(
            Auth::Basic {
                username: "Aladdin".into(),
                password: "open sesame".into()
            }
            .effect(&mut render),
            Some(AuthEffect::Header(
                "Authorization".into(),
                "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==".into()
            ))
        );
        assert_eq!(
            Auth::ApiKey {
                key: "api_key".into(),
                value: "{{t}}".into(),
                location: KeyLocation::Query
            }
            .effect(&mut render),
            Some(AuthEffect::Query("api_key".into(), "abc".into()))
        );
        assert_eq!(Auth::Inherit.effect(&mut render), None);
        for i in 0..Auth::KINDS.len() {
            assert_eq!(Auth::of_kind(i).kind_index(), i);
        }
    }
}
