use crate::classifier;
use crate::models::{RawWindow, Rect, WindowSample};

pub fn compute_visible_windows(windows: Vec<RawWindow>) -> Vec<WindowSample> {
    let mut covered: Vec<Rect> = Vec::new();
    let mut samples: Vec<WindowSample> = Vec::new();
    let mut total_visible_area = 0_i64;

    for window in windows {
        let visible_area = uncovered_area(window.rect, &covered);

        if visible_area > 0 {
            total_visible_area += visible_area;
            samples.push(WindowSample {
                category: classifier::classify(
                    &window.app_name,
                    &window.title,
                    &window.process_path,
                ),
                app_name: window.app_name,
                window_title: window.title,
                process_path: window.process_path,
                pid: window.pid,
                rect: window.rect,
                visible_area,
                visible_share: 0.0,
                focused: window.focused,
            });
        }

        if window.rect.area() > 0 {
            covered.push(window.rect);
        }
    }

    if total_visible_area > 0 {
        for sample in &mut samples {
            sample.visible_share = sample.visible_area as f64 / total_visible_area as f64;
        }
    }

    samples
}

fn uncovered_area(rect: Rect, covered: &[Rect]) -> i64 {
    let mut fragments = vec![rect];

    for cover in covered {
        let mut next = Vec::new();

        for fragment in fragments {
            if let Some(overlap) = fragment.intersection(*cover) {
                next.extend(split_around_overlap(fragment, overlap));
            } else {
                next.push(fragment);
            }
        }

        fragments = next;

        if fragments.is_empty() {
            break;
        }
    }

    fragments.into_iter().map(Rect::area).sum()
}

fn split_around_overlap(rect: Rect, overlap: Rect) -> Vec<Rect> {
    let mut pieces = Vec::with_capacity(4);

    if rect.top < overlap.top {
        pieces.push(Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: overlap.top,
        });
    }

    if overlap.bottom < rect.bottom {
        pieces.push(Rect {
            left: rect.left,
            top: overlap.bottom,
            right: rect.right,
            bottom: rect.bottom,
        });
    }

    if rect.left < overlap.left {
        pieces.push(Rect {
            left: rect.left,
            top: overlap.top,
            right: overlap.left,
            bottom: overlap.bottom,
        });
    }

    if overlap.right < rect.right {
        pieces.push(Rect {
            left: overlap.right,
            top: overlap.top,
            right: rect.right,
            bottom: overlap.bottom,
        });
    }

    pieces
        .into_iter()
        .filter(|piece| piece.area() > 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtracts_covering_rectangles() {
        let base = Rect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        };
        let cover = Rect {
            left: 0,
            top: 0,
            right: 50,
            bottom: 100,
        };

        assert_eq!(uncovered_area(base, &[cover]), 5_000);
    }
}
