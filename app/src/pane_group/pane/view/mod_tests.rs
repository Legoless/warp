use pathfinder_color::ColorU;
use warp_core::ui::color::blend::Blend;
use warp_core::ui::color::coloru_with_opacity;
use warp_core::ui::theme::Fill as ThemeFill;

use super::*;

#[test]
fn pane_activity_background_blends_activity_tint_into_theme_background() {
    let background = ThemeFill::Solid(ColorU::new(10, 20, 30, 255));
    let activity_color = ColorU::new(255, 0, 0, 255);

    assert_eq!(
        pane_activity_background(background, Some(activity_color), None, true),
        Some(background.blend(&ThemeFill::Solid(coloru_with_opacity(
            activity_color,
            PANE_ACTIVITY_TINT_OPACITY,
        ))))
    );
}

#[test]
fn pane_activity_background_is_absent_without_activity_color() {
    let background = ThemeFill::Solid(ColorU::new(10, 20, 30, 255));

    assert_eq!(pane_activity_background(background, None, None, true), None);
}

#[test]
fn pane_activity_background_is_absent_when_background_tint_is_disabled() {
    let background = ThemeFill::Solid(ColorU::new(10, 20, 30, 255));
    let activity_color = ColorU::new(255, 0, 0, 255);
    let inactive_overlay = ThemeFill::Solid(ColorU::new(255, 255, 255, 25));

    assert_eq!(
        pane_activity_background(
            background,
            Some(activity_color),
            Some(inactive_overlay),
            false
        ),
        None
    );
}

#[test]
fn pane_activity_background_blends_inactive_overlay_into_background() {
    let background = ThemeFill::Solid(ColorU::new(10, 20, 30, 255));
    let activity_color = ColorU::new(255, 0, 0, 255);
    let inactive_overlay = ThemeFill::Solid(ColorU::new(255, 255, 255, 25));

    let activity_background = background.blend(&ThemeFill::Solid(coloru_with_opacity(
        activity_color,
        PANE_ACTIVITY_TINT_OPACITY,
    )));
    assert_eq!(
        pane_activity_background(
            background,
            Some(activity_color),
            Some(inactive_overlay),
            true
        ),
        Some(activity_background.blend(&inactive_overlay))
    );
}

#[test]
fn pane_preserved_content_activity_chrome_tints_with_activity_color() {
    let background = ThemeFill::Solid(ColorU::new(10, 20, 30, 255));
    let activity_color = ColorU::new(255, 0, 0, 255);

    let expected =
        ThemeFill::Solid(background.into_solid_bias_top_color()).blend(&ThemeFill::Solid(
            coloru_with_opacity(activity_color, PANE_HEADER_ACTIVITY_TINT_OPACITY),
        ));
    assert_eq!(
        pane_preserved_content_activity_chrome(background, Some(activity_color), None),
        Some(expected)
    );
}

#[test]
fn pane_preserved_content_activity_chrome_blends_tint_and_inactive_dim() {
    let background = ThemeFill::Solid(ColorU::new(10, 20, 30, 255));
    let activity_color = ColorU::new(255, 0, 0, 255);
    let inactive_overlay = ThemeFill::Solid(ColorU::new(255, 255, 255, 25));

    let tinted = ThemeFill::Solid(background.into_solid_bias_top_color()).blend(&ThemeFill::Solid(
        coloru_with_opacity(activity_color, PANE_HEADER_ACTIVITY_TINT_OPACITY),
    ));
    assert_eq!(
        pane_preserved_content_activity_chrome(
            background,
            Some(activity_color),
            Some(inactive_overlay)
        ),
        Some(tinted.blend(&inactive_overlay))
    );
}

#[test]
fn pane_preserved_content_activity_chrome_is_absent_without_activity_color() {
    let background = ThemeFill::Solid(ColorU::new(10, 20, 30, 255));
    let inactive_overlay = ThemeFill::Solid(ColorU::new(255, 255, 255, 25));

    assert_eq!(
        pane_preserved_content_activity_chrome(background, None, Some(inactive_overlay)),
        None
    );
    assert_eq!(
        pane_preserved_content_activity_chrome(background, None, None),
        None
    );
}
