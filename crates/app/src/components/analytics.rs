//! Analytics dashboard, priority bar chart, and interactive 2D pan/zoom parametric canvas.

use crate::{ExportFormat, ScreenConstraints, TemplateApp};
use eframe::egui;
use shared::{ColorblindMode, Priority};
use spodeian_ui::PerformanceMetricCard;

fn priority_bar_colors(mode: ColorblindMode) -> [egui::Color32; 4] {
    match mode {
        ColorblindMode::None => [
            egui::Color32::from_rgb(239, 68, 68),
            egui::Color32::from_rgb(249, 115, 22),
            egui::Color32::from_rgb(56, 189, 248),
            egui::Color32::from_rgb(148, 163, 184),
        ],
        ColorblindMode::Deuteranopia => [
            egui::Color32::from_rgb(217, 95, 2),
            egui::Color32::from_rgb(230, 171, 2),
            egui::Color32::from_rgb(59, 130, 246),
            egui::Color32::from_rgb(148, 163, 184),
        ],
        ColorblindMode::Protanopia => [
            egui::Color32::from_rgb(234, 88, 12),
            egui::Color32::from_rgb(202, 138, 4),
            egui::Color32::from_rgb(37, 99, 235),
            egui::Color32::from_rgb(148, 163, 184),
        ],
        ColorblindMode::Tritanopia => [
            egui::Color32::from_rgb(244, 63, 94),
            egui::Color32::from_rgb(251, 113, 133),
            egui::Color32::from_rgb(14, 165, 233),
            egui::Color32::from_rgb(148, 163, 184),
        ],
        ColorblindMode::Monochrome => [
            egui::Color32::from_rgb(248, 250, 252),
            egui::Color32::from_rgb(203, 213, 225),
            egui::Color32::from_rgb(148, 163, 184),
            egui::Color32::from_rgb(100, 116, 139),
        ],
    }
}

/// Renders KPI metrics, colorblind-safe priority breakdown chart, and an interactive 2D
/// pan/zoom parametric signal viewport.
pub fn render_analytics_page(
    app: &mut TemplateApp,
    ui: &mut egui::Ui,
    constraints: &ScreenConstraints,
) {
    let total = app.state.collection.total_count();
    let completed = app.state.collection.completed_count();
    let high_crit = app.state.collection.high_priority_count();
    let pct = (app.state.collection.completion_ratio() * 100.0).round();
    let counts = app.state.collection.priority_counts();
    let cb_mode = app.state.config.accessibility.colorblind_mode;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.heading("📈 Analytics & Interactive 2D Canvas");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("📄 Printable HTML Report")
                        .on_hover_text("Open self-contained HTML5 report export dialog")
                        .clicked()
                    {
                        app.selected_export_format = ExportFormat::Html;
                        app.open_export_dialog(ExportFormat::Html);
                    }
                    if ui
                        .button("🖼 Export SVG Chart")
                        .on_hover_text("Open standalone vector SVG chart export dialog")
                        .clicked()
                    {
                        app.selected_export_format = ExportFormat::Svg;
                        app.open_export_dialog(ExportFormat::Svg);
                    }
                });
            });
            ui.add_space(10.0);

            ui.horizontal_wrapped(|ui| {
                PerformanceMetricCard::new("Total Items", total.to_string())
                    .tooltip("Total managed items in the active working set")
                    .show(ui);
                PerformanceMetricCard::new("Completion", format!("{pct:.0}%"))
                    .unit(format!("{completed}/{total}"))
                    .tooltip("Share of completed items")
                    .show(ui);
                PerformanceMetricCard::new("High / Critical", high_crit.to_string())
                    .warning(high_crit > 0)
                    .tooltip("Pending High or Critical priority items")
                    .show(ui);
                PerformanceMetricCard::new("Vision Profile", format!("{cb_mode:?}"))
                    .tooltip("Active colorblind compensation palette from Accessibility settings")
                    .show(ui);
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Priority Distribution").strong().size(15.0));
                ui.add_space(6.0);

                let chart_w = ui.available_width().max(220.0);
                let chart_h = if constraints.is_mobile { 170.0 } else { 200.0 };
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(chart_w, chart_h), egui::Sense::hover());
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Other,
                        true,
                        format!(
                            "Priority chart: Critical {}, High {}, Medium {}, Low {}",
                            counts[0], counts[1], counts[2], counts[3]
                        ),
                    )
                });

                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
                let palette = priority_bar_colors(cb_mode);
                let labels = [
                    Priority::Critical.label(),
                    Priority::High.label(),
                    Priority::Medium.label(),
                    Priority::Low.label(),
                ];
                let max_count = counts.iter().copied().max().unwrap_or(1).max(1) as f32;
                let col_w = rect.width() / 4.0;
                let bar_w = (col_w * 0.48).clamp(24.0, 96.0);
                let bar_top = rect.top() + 26.0;
                let bar_bottom = rect.bottom() - 28.0;
                let max_bar_h = (bar_bottom - bar_top).max(16.0);
                let text_color = ui.visuals().text_color();

                for idx in 0..4 {
                    let count = counts[idx];
                    let cx = rect.left() + col_w * (idx as f32 + 0.5);
                    let frac = count as f32 / max_count;
                    let h = if count == 0 {
                        4.0
                    } else {
                        (frac * max_bar_h).max(10.0)
                    };
                    let bar_rect = egui::Rect::from_min_max(
                        egui::pos2(cx - bar_w * 0.5, bar_bottom - h),
                        egui::pos2(cx + bar_w * 0.5, bar_bottom),
                    );
                    painter.rect_filled(bar_rect, 4.0, palette[idx]);
                    painter.text(
                        egui::pos2(cx, bar_rect.top() - 6.0),
                        egui::Align2::CENTER_BOTTOM,
                        count.to_string(),
                        egui::FontId::proportional(13.0),
                        text_color,
                    );
                    painter.text(
                        egui::pos2(cx, bar_bottom + 8.0),
                        egui::Align2::CENTER_TOP,
                        labels[idx],
                        egui::FontId::proportional(12.0),
                        ui.visuals().weak_text_color(),
                    );
                }
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Interactive 2D Parametric Viewport (Drag to Pan • Scroll/Buttons to Zoom)")
                            .strong()
                            .size(15.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Reset View").clicked() {
                            app.canvas_pan = egui::Vec2::ZERO;
                            app.canvas_zoom = 1.0;
                        }
                        if ui.button("+").on_hover_text("Zoom in").clicked() {
                            app.canvas_zoom = (app.canvas_zoom * 1.25).clamp(0.25, 4.0);
                        }
                        if ui.button("-").on_hover_text("Zoom out").clicked() {
                            app.canvas_zoom = (app.canvas_zoom / 1.25).clamp(0.25, 4.0);
                        }
                        ui.label(format!("{:.0}%", app.canvas_zoom * 100.0));
                    });
                });
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label("Freq (Hz):");
                    ui.add(
                        egui::Slider::new(&mut app.preview_config.frequency_hz, 0.5..=12.0)
                            .step_by(0.25),
                    );
                    ui.label("Damping:");
                    ui.add(
                        egui::Slider::new(&mut app.preview_config.damping, 0.0..=2.5)
                            .step_by(0.05),
                    );
                    ui.label("2nd Harmonic:");
                    ui.add(
                        egui::Slider::new(&mut app.preview_config.harmonic_ratio, 0.0..=1.0)
                            .step_by(0.05),
                    );
                });
                ui.add_space(6.0);

                let canvas_w = ui.available_width().max(220.0);
                let canvas_h = if constraints.is_mobile { 210.0 } else { 250.0 };
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(canvas_w, canvas_h),
                    egui::Sense::click_and_drag(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Other,
                        true,
                        "Interactive 2D damped harmonic signal plot",
                    )
                });

                if response.dragged() {
                    app.canvas_pan += response.drag_delta();
                }
                if response.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll.abs() > 0.1 {
                        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                        app.canvas_zoom = (app.canvas_zoom * factor).clamp(0.25, 4.0);
                    }
                }

                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
                painter.rect_stroke(
                    rect,
                    8.0,
                    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                    egui::StrokeKind::Inside,
                );

                let origin = rect.center() + app.canvas_pan;
                let grid_stroke = egui::Stroke::new(
                    1.0,
                    ui.visuals()
                        .widgets
                        .noninteractive
                        .bg_stroke
                        .color
                        .linear_multiply(0.5),
                );
                let step = 40.0 * app.canvas_zoom;
                if step >= 8.0 {
                    let mut x = origin.x.rem_euclid(step) + rect.left();
                    while x <= rect.right() {
                        painter.line_segment(
                            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                            grid_stroke,
                        );
                        x += step;
                    }
                    let mut y = origin.y.rem_euclid(step) + rect.top();
                    while y <= rect.bottom() {
                        painter.line_segment(
                            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                            grid_stroke,
                        );
                        y += step;
                    }
                }

                let axis_stroke = egui::Stroke::new(1.2, ui.visuals().weak_text_color());
                if origin.y >= rect.top() && origin.y <= rect.bottom() {
                    painter.line_segment(
                        [
                            egui::pos2(rect.left(), origin.y),
                            egui::pos2(rect.right(), origin.y),
                        ],
                        axis_stroke,
                    );
                }

                let half_w = rect.width() * 0.44 * app.canvas_zoom;
                let half_h = rect.height() * 0.34 * app.canvas_zoom;
                let samples = app.preview_config.sample_points(160);
                let points: Vec<egui::Pos2> = samples
                    .into_iter()
                    .map(|(t, y)| {
                        let px = origin.x + (t - 0.5) * 2.0 * half_w;
                        let py = origin.y - y * half_h;
                        egui::pos2(px, py)
                    })
                    .collect();

                let curve_color = priority_bar_colors(cb_mode)[2];
                painter.add(egui::Shape::line(
                    points,
                    egui::Stroke::new(2.2, curve_color),
                ));
            });

            ui.add_space(8.0);
            let compute = shared::ComputeRuntimeSummary::current();
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Renderer:").strong());
                ui.label(crate::active_renderer_label());
                ui.separator();
                ui.label(egui::RichText::new("Compute:").strong());
                ui.label(format!(
                    "{} ({} threads)",
                    compute.runtime_label, compute.parallel_threads
                ));
            });
            ui.collapsing(
                "Bundled WebGPU WGSL Compute Kernel (PARAMETRIC_HARMONIC_WGSL_SHADER)",
                |ui| {
                    ui.label(
                        egui::RichText::new(shared::PARAMETRIC_HARMONIC_WGSL_SHADER)
                            .monospace()
                            .size(12.0),
                    );
                },
            );

            ui.add_space(12.0);
        });
}
