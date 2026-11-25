//! The look of the interface (ADR 0012): black on white like a drawing,
//! Arial, and our own icons (ADR 0004) read from `assets/icons`.

use std::{path::PathBuf, sync::Arc};

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Painter, Pos2, Rect, Response,
    Sense, Shadow, Shape, Stroke, StrokeKind, Ui, Vec2, pos2,
};

const INK: Color32 = Color32::BLACK;
const PAPER: Color32 = Color32::WHITE;
/// The light grey of separators and of a button under the pointer.
const FAINT: Color32 = Color32::from_gray(225);

/// Sets the look of the interface: black on white, square corners, no
/// shadows, and Arial when the system has it.
pub fn style(ctx: &egui::Context) {
    if let Some(arial) = arial() {
        let mut fonts = FontDefinitions::default();
        fonts
            .font_data
            .insert("arial".to_owned(), Arc::new(FontData::from_owned(arial)));
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.insert(0, "arial".to_owned());
        }
        ctx.set_fonts(fonts);
    }
    ctx.set_theme(egui::Theme::Light);
    ctx.style_mut_of(egui::Theme::Light, |style| {
        let v = &mut style.visuals;
        v.panel_fill = PAPER;
        v.window_fill = PAPER;
        v.extreme_bg_color = PAPER;
        v.faint_bg_color = Color32::from_gray(245);
        v.code_bg_color = Color32::from_gray(240);
        v.hyperlink_color = INK;
        v.weak_text_color = Some(Color32::from_gray(90));
        v.window_stroke = Stroke::new(1.0, INK);
        v.window_corner_radius = CornerRadius::ZERO;
        v.menu_corner_radius = CornerRadius::ZERO;
        v.window_shadow = Shadow::NONE;
        v.popup_shadow = Shadow::NONE;
        v.selection.bg_fill = INK;
        v.selection.stroke = Stroke::new(1.0, PAPER);
        let w = &mut v.widgets;
        for (state, fill, line, text) in [
            (&mut w.noninteractive, PAPER, Stroke::new(1.0, FAINT), INK),
            (&mut w.inactive, PAPER, Stroke::new(1.0, INK), INK),
            (&mut w.hovered, FAINT, Stroke::new(1.0, INK), INK),
            // Pressed: a darker grey. egui also draws strong text in this
            // colour, so it stays black.
            (
                &mut w.active,
                Color32::from_gray(190),
                Stroke::new(1.0, INK),
                INK,
            ),
            (&mut w.open, FAINT, Stroke::new(1.0, INK), INK),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = line;
            state.fg_stroke = Stroke::new(state.fg_stroke.width, text);
            state.corner_radius = CornerRadius::ZERO;
            state.expansion = 0.0;
        }
    });
}

/// Arial, read where the system keeps it: the font is not ours to ship.
/// Liberation Sans, drawn to the same widths, stands in on Linux.
// ponytail: a fixed list of places and one font; a font setting once the
// interface has preferences.
fn arial() -> Option<Vec<u8>> {
    let mut places: Vec<PathBuf> = Vec::new();
    if let Some(windows) = std::env::var_os("WINDIR") {
        places.push(PathBuf::from(windows).join("Fonts").join("arial.ttf"));
    }
    places.extend(
        [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/Library/Fonts/Arial.ttf",
            "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ]
        .map(PathBuf::from),
    );
    places.iter().find_map(|p| std::fs::read(p).ok())
}

/// One stroke of an icon, in units of its 24 x 24 grid.
#[derive(Debug, PartialEq)]
struct Line {
    points: Vec<Pos2>,
    closed: bool,
    /// Dash and gap lengths in turn; empty for a solid line.
    dash: Vec<f32>,
}

/// The numbers of a list such as `points` or a dash array.
fn numbers(text: &str) -> Result<Vec<f32>, String> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().map_err(|_| format!("not a number: {s}")))
        .collect()
}

/// The value of an attribute of a tag.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let mut from = 0;
    while let Some(i) = tag[from..].find(&key) {
        let at = from + i;
        if tag[..at].ends_with(char::is_whitespace) {
            let rest = &tag[at + key.len()..];
            return rest.find('"').map(|end| &rest[..end]);
        }
        from = at + key.len();
    }
    None
}

fn number(tag: &str, name: &str) -> Result<f32, String> {
    let value = attr(tag, name).ok_or_else(|| format!("{name} missing"))?;
    value.parse().map_err(|_| format!("{name}: not a number"))
}

/// The points of a list of pairs.
fn pairs(values: &[f32]) -> Vec<Pos2> {
    values
        .as_chunks()
        .0
        .iter()
        .map(|&[x, y]| pos2(x, y))
        .collect()
}

/// Reads an icon written in the subset of the style guide
/// (`assets/icons/README.md`); anything else is an error.
fn parse(svg: &str) -> Result<Vec<Line>, String> {
    let mut lines = Vec::new();
    for piece in svg.split('<').skip(1) {
        let tag = piece.split('>').next().unwrap_or_default();
        let name = tag.split_whitespace().next().unwrap_or_default();
        if name.starts_with(['/', '?', '!']) || name == "svg" {
            continue;
        }
        let dash = attr(tag, "stroke-dasharray").map_or(Ok(Vec::new()), numbers)?;
        let mut add = |points: Vec<Pos2>, closed| {
            lines.push(Line {
                points,
                closed,
                dash: dash.clone(),
            });
        };
        let n = |key| number(tag, key);
        match name {
            "line" => add(
                vec![pos2(n("x1")?, n("y1")?), pos2(n("x2")?, n("y2")?)],
                false,
            ),
            "rect" => {
                let (x, y, w, h) = (n("x")?, n("y")?, n("width")?, n("height")?);
                add(
                    vec![
                        pos2(x, y),
                        pos2(x + w, y),
                        pos2(x + w, y + h),
                        pos2(x, y + h),
                    ],
                    true,
                );
            }
            "circle" => {
                let (c, r) = (pos2(n("cx")?, n("cy")?), n("r")?);
                let steps = 32;
                let points = (0..steps)
                    .map(|i| c + Vec2::angled(i as f32 / steps as f32 * std::f32::consts::TAU) * r)
                    .collect();
                add(points, true);
            }
            "polyline" | "polygon" => {
                let points = pairs(&numbers(attr(tag, "points").unwrap_or_default())?);
                add(points, name == "polygon");
            }
            "path" => {
                for (points, closed) in path(attr(tag, "d").unwrap_or_default())? {
                    add(points, closed);
                }
            }
            other => return Err(format!("element not in the subset: {other}")),
        }
    }
    Ok(lines)
}

/// The sub-paths of a path made of absolute M, L, C, Q and Z.
fn path(d: &str) -> Result<Vec<(Vec<Pos2>, bool)>, String> {
    let mut spaced = String::with_capacity(d.len() * 2);
    for c in d.chars() {
        if c.is_ascii_alphabetic() {
            spaced.push(' ');
            spaced.push(c);
            spaced.push(' ');
        } else {
            spaced.push(c);
        }
    }
    let mut out = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let mut tokens = spaced.split_whitespace().peekable();
    let flush = |current: &mut Vec<Pos2>, out: &mut Vec<(Vec<Pos2>, bool)>, closed| {
        if current.len() > 1 {
            out.push((std::mem::take(current), closed));
        }
        current.clear();
    };
    while let Some(command) = tokens.next() {
        let mut values = Vec::new();
        while let Some(t) = tokens.peek() {
            if t.starts_with(|c: char| c.is_ascii_alphabetic()) {
                break;
            }
            values.extend(numbers(t)?);
            tokens.next();
        }
        let points = pairs(&values);
        let last = current.last().copied().unwrap_or(Pos2::ZERO);
        match command {
            "M" => {
                flush(&mut current, &mut out, false);
                current.extend(points);
            }
            "L" => current.extend(points),
            "C" => {
                for &[a, b, c] in points.as_chunks().0 {
                    let from = current.last().copied().unwrap_or(last);
                    current.extend(curve(&[from, a, b, c]));
                }
            }
            "Q" => {
                for &[a, b] in points.as_chunks().0 {
                    let from = current.last().copied().unwrap_or(last);
                    current.extend(curve(&[from, a, b]));
                }
            }
            "Z" => {
                let start = current.first().copied();
                flush(&mut current, &mut out, true);
                // A path may go on from where it closed.
                current.extend(start);
            }
            other => return Err(format!("path command not in the subset: {other}")),
        }
    }
    flush(&mut current, &mut out, false);
    Ok(out)
}

/// A BÃ©zier curve, quadratic or cubic, as twelve straight steps (the
/// first point left out).
fn curve(control: &[Pos2]) -> Vec<Pos2> {
    (1..=12)
        .map(|i| {
            let t = i as f32 / 12.0;
            // De Casteljau: shrink the control points to one.
            let mut p = control.to_vec();
            while p.len() > 1 {
                p = p.windows(2).map(|w| w[0].lerp(w[1], t)).collect();
            }
            p[0]
        })
        .collect()
}

/// Draws an icon in a square, in one colour.
pub(crate) fn paint(painter: &Painter, rect: Rect, svg: &str, color: Color32) {
    // ponytail: read again at every frame; a few small files, well under a
    // millisecond. Keep them read once if the tool bar grows to hundreds.
    let Ok(lines) = parse(svg) else {
        return;
    };
    let scale = rect.width() / 24.0;
    let stroke = Stroke::new((1.5 * scale).max(1.0), color);
    for line in lines {
        let mut points: Vec<Pos2> = line
            .points
            .iter()
            .map(|p| rect.min + p.to_vec2() * scale)
            .collect();
        if line.closed {
            points.extend(points.first().copied());
        }
        if line.dash.is_empty() {
            painter.add(Shape::line(points, stroke));
        } else {
            let at = |i: usize| line.dash[i % line.dash.len()] * scale;
            let count = if line.dash.len() % 2 == 0 {
                line.dash.len() / 2
            } else {
                line.dash.len()
            };
            let dashes: Vec<f32> = (0..count).map(|i| at(2 * i)).collect();
            let gaps: Vec<f32> = (0..count).map(|i| at(2 * i + 1)).collect();
            painter.extend(Shape::dashed_line_with_offset(
                &points, stroke, &dashes, &gaps, 0.0,
            ));
        }
    }
}

/// Side of an icon on a button, in points.
const ICON: f32 = 20.0;

/// A square button showing an icon: black on white, white on black when
/// it is on. Its name shows on hover.
pub(crate) fn icon_button(ui: &mut Ui, on: bool, svg: &str, name: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(ICON + 6.0), Sense::click());
    let visuals = ui.style().interact_selectable(&response, on);
    if on || response.hovered() {
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, Stroke::NONE, StrokeKind::Inside);
    }
    paint(ui.painter(), rect.shrink(3.0), svg, visuals.fg_stroke.color);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), on, name)
    });
    response.on_hover_text(name)
}

/// A row of the tool bar: the icon, then the tool's name; black on white,
/// white on black when it is the tool in use.
pub(crate) fn icon_row(ui: &mut Ui, on: bool, svg: &str, name: &str) -> Response {
    let height = ICON + 6.0;
    let width = ui.available_width().max(height);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let visuals = ui.style().interact_selectable(&response, on);
    if on || response.hovered() {
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, Stroke::NONE, StrokeKind::Inside);
    }
    let icon = Rect::from_min_size(rect.min + Vec2::splat(3.0), Vec2::splat(ICON));
    paint(ui.painter(), icon, svg, visuals.fg_stroke.color);
    ui.painter().text(
        pos2(icon.max.x + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::TextStyle::Button.resolve(ui.style()),
        visuals.fg_stroke.color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::RadioButton, ui.is_enabled(), on, name)
    });
    response
}

/// The icons of the catalogue, by file name.
pub(crate) mod icons {
    macro_rules! icons {
        ($($name:ident = $file:literal),* $(,)?) => {
            $(pub(crate) const $name: &str =
                include_str!(concat!("../../../assets/icons/", $file));)*
        };
    }
    icons! {
        SELECT = "select.svg",
        WALL = "wall.svg",
        SLAB = "slab.svg",
        COLUMN = "column.svg",
        BEAM = "beam.svg",
        WINDOW = "window.svg",
        DOOR = "door.svg",
        ROOF = "roof.svg",
        STAIR = "stair.svg",
        TERRAIN = "terrain.svg",
        GUIDE = "guide.svg",
        SNAP_POINTS = "snap-points.svg",
        SNAP_MIDPOINTS = "snap-midpoints.svg",
        SNAP_INTERSECTIONS = "snap-intersections.svg",
        SNAP_EDGES = "snap-edges.svg",
        SNAP_ANGLES = "snap-angles.svg",
        SNAP_ALIGNMENTS = "snap-alignments.svg",
        SNAP_GUIDES = "snap-guides.svg",
        SNAP_HALFWAY = "snap-halfway.svg",
        SNAP_REFERENCE = "snap-reference.svg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_shape_of_the_subset() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor">
  <line x1="2" y1="12" x2="22" y2="12"/>
  <rect x="4" y="4" width="16" height="8" stroke-dasharray="2 2"/>
  <circle cx="12" cy="12" r="1.5"/>
  <polyline points="2,2 4,4 6,2"/>
  <polygon points="2 2 4 4 6 2"/>
  <path d="M2 20L12 20Q16 20 16 16C16 12 20 12 22 12Z"/>
</svg>"#;
        let lines = parse(svg).unwrap_or_default();
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0].points, vec![pos2(2.0, 12.0), pos2(22.0, 12.0)]);
        assert!(lines[1].closed && lines[1].dash == vec![2.0, 2.0]);
        assert!(
            lines[2]
                .points
                .iter()
                .all(|p| (p.distance(pos2(12.0, 12.0)) - 1.5).abs() < 1e-4)
        );
        assert!(!lines[3].closed && lines[4].closed);
        let path = &lines[5];
        assert!(path.closed);
        assert_eq!(path.points.len(), 2 + 12 + 12);
        assert_eq!(path.points.last(), Some(&pos2(22.0, 12.0)));
        assert_eq!(path.points[13], pos2(16.0, 16.0));
    }

    #[test]
    fn refuses_what_is_not_in_the_subset() {
        assert!(parse("<svg><g></g></svg>").is_err());
        assert!(parse(r#"<svg><path d="M0 0 A 2 2 0 0 1 4 4"/></svg>"#).is_err());
        assert!(parse(r#"<svg><line x1="0" y1="0" x2="4"/></svg>"#).is_err());
    }

    #[test]
    fn an_attribute_is_not_found_inside_another() {
        assert_eq!(attr(r#"line x1="1" x="2""#, "x"), Some("2"));
        assert_eq!(attr(r#"rect stroke-width="3""#, "width"), None);
    }

    #[test]
    fn every_icon_of_the_catalogue_reads_and_stays_on_the_grid() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icons");
        let files: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{dir}: {e}"))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "svg"))
            .collect();
        assert!(files.len() >= 20, "only {} icons in {dir}", files.len());
        for path in files {
            let file = path.display();
            let svg = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
            let lines = parse(&svg).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert!(!lines.is_empty(), "{file}: empty");
            for p in lines.iter().flat_map(|l| &l.points) {
                assert!(
                    (0.0..=24.0).contains(&p.x) && (0.0..=24.0).contains(&p.y),
                    "{file}: {p:?} off the grid"
                );
            }
        }
    }
}
