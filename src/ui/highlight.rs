//! Text colours: `{{variables}}` in fields, JSON in bodies, search matches
//! in responses.

use std::sync::Arc;

use domain::{JsonToken, Segment, Variables, segments, tokenize_json};
use egui::text::{LayoutJob, TextFormat};
use egui::util::cache::{ComputerMut, FrameCache};
use egui::{Color32, FontId, Galley, TextStyle, Ui};

use super::theme;

/// Bodies larger than this are shown without colours (it would cost every
/// frame they are on screen).
pub const HIGHLIGHT_LIMIT: usize = 256 * 1024;

fn format(font: &FontId, color: Color32) -> TextFormat {
    TextFormat::simple(font.clone(), color)
}

/// `{{name}}` green when defined, red otherwise; the rest as usual.
/// Single-line fields don't wrap (they scroll).
pub fn template_layouter(
    variables: &Variables,
    style: TextStyle,
    multiline: bool,
) -> impl FnMut(&Ui, &str, f32) -> Arc<Galley> + '_ {
    move |ui, text, wrap_width| {
        let font = style.resolve(ui.style());
        let normal = ui.visuals().text_color();
        let mut job = LayoutJob::default();
        for segment in segments(text) {
            match segment {
                Segment::Text(range) => job.append(&text[range], 0.0, format(&font, normal)),
                Segment::Variable { range, name } => {
                    let color = theme::variable(ui, variables.contains(&name));
                    job.append(&text[range], 0.0, format(&font, color));
                }
            }
        }
        job.wrap.max_width = if multiline { wrap_width } else { f32::INFINITY };
        ui.fonts(|f| f.layout_job(job))
    }
}

/// JSON colours, with placeholders coloured by `variables` (request bodies).
pub fn json_layouter(variables: &Variables) -> impl FnMut(&Ui, &str, f32) -> Arc<Galley> + '_ {
    move |ui, text, wrap_width| {
        let mut job = if text.len() > HIGHLIGHT_LIMIT {
            plain_job(ui, text)
        } else {
            json_job(
                ui.visuals().dark_mode,
                ui.visuals().text_color(),
                text,
                &mono(ui),
                |name| Some(theme::variable(ui, variables.contains(name))),
            )
        };
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    }
}

/// Response bodies, cached (the same body is drawn every frame): JSON
/// colours, or the matches of `find` highlighted.
pub fn response_layouter(json: bool, find: &str) -> impl FnMut(&Ui, &str, f32) -> Arc<Galley> + '_ {
    move |ui, text, wrap_width| {
        let mut job = if text.len() <= HIGHLIGHT_LIMIT && (json || !find.is_empty()) {
            let key = ResponseKey {
                dark_mode: ui.visuals().dark_mode,
                normal: ui.visuals().text_color(),
                mark: theme::accent(ui).gamma_multiply(0.45),
                size: mono(ui).size.to_bits(),
                json,
                find,
                text,
            };
            ui.memory_mut(|m| {
                m.caches
                    .cache::<FrameCache<LayoutJob, ResponseHighlighter>>()
                    .get(key)
            })
        } else {
            plain_job(ui, text)
        };
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    }
}

/// Where `find` occurs in `text`, ignoring ASCII case.
pub fn matches(text: &str, find: &str) -> Vec<std::ops::Range<usize>> {
    if find.is_empty() {
        return Vec::new();
    }
    let haystack = text.to_ascii_lowercase();
    let needle = find.to_ascii_lowercase();
    haystack
        .match_indices(&needle)
        .map(|(i, m)| i..i + m.len())
        .collect()
}

fn mono(ui: &Ui) -> FontId {
    TextStyle::Monospace.resolve(ui.style())
}

fn plain_job(ui: &Ui, text: &str) -> LayoutJob {
    LayoutJob::simple(
        text.to_owned(),
        mono(ui),
        ui.visuals().text_color(),
        f32::INFINITY,
    )
}

fn json_job(
    dark_mode: bool,
    normal: Color32,
    text: &str,
    font: &FontId,
    placeholder: impl Fn(&str) -> Option<Color32>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (token, range) in tokenize_json(text) {
        let piece = &text[range];
        let color = match token {
            JsonToken::Placeholder => {
                placeholder(piece.trim_start_matches('{').trim_end_matches('}').trim())
            }
            other => theme::json(dark_mode, other),
        };
        job.append(piece, 0.0, format(font, color.unwrap_or(normal)));
    }
    job
}

#[derive(Clone, Copy, Hash)]
struct ResponseKey<'a> {
    dark_mode: bool,
    normal: Color32,
    mark: Color32,
    size: u32,
    json: bool,
    find: &'a str,
    text: &'a str,
}

#[derive(Default)]
struct ResponseHighlighter;

impl ComputerMut<ResponseKey<'_>, LayoutJob> for ResponseHighlighter {
    fn compute(&mut self, key: ResponseKey<'_>) -> LayoutJob {
        let font = FontId::monospace(f32::from_bits(key.size));
        let base = if key.json {
            json_job(key.dark_mode, key.normal, key.text, &font, |_| None)
        } else {
            LayoutJob::single_section(key.text.to_owned(), format(&font, key.normal))
        };
        let found = matches(key.text, key.find);
        if found.is_empty() {
            return base;
        }
        // Re-cut the sections at the matches' edges, marking those inside.
        let mut job = LayoutJob::default();
        for section in &base.sections {
            let mut start = section.byte_range.start;
            let end = section.byte_range.end;
            let mut edges: Vec<usize> = found
                .iter()
                .flat_map(|r| [r.start, r.end])
                .filter(|&e| e > start && e < end)
                .collect();
            edges.push(end);
            for edge in edges {
                let inside = found.iter().any(|r| r.start <= start && start < r.end);
                let mut fmt = section.format.clone();
                if inside {
                    fmt.background = key.mark;
                }
                job.append(&key.text[start..edge], 0.0, fmt);
                start = edge;
            }
        }
        job
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_matches_ignoring_case() {
        assert_eq!(matches("Ab ab AB", "ab"), [0..2, 3..5, 6..8]);
        assert!(matches("abc", "").is_empty());
    }

    #[test]
    fn marks_matches_across_sections() {
        let key = ResponseKey {
            dark_mode: true,
            normal: Color32::WHITE,
            mark: Color32::RED,
            size: 13.0_f32.to_bits(),
            json: true,
            find: "id",
            text: r#"{"id": 7, "uid": "hidden"}"#,
        };
        let job = ResponseHighlighter.compute(key);
        let marked: String = job
            .sections
            .iter()
            .filter(|s| s.format.background == Color32::RED)
            .map(|s| &job.text[s.byte_range.clone()])
            .collect();
        assert_eq!(marked, "ididid");
        assert_eq!(job.text, key.text);
    }
}
