//! Floating "+N" indicators for Photon-driven global counter deltas.
//!
//! Mount beside [`super::counter_metrics::CounterMetrics`] and pass the
//! confirmed global signal from the live page. When Photon refetch (or a local
//! flush) raises that value, hydrate builds spawn a `web_sys` span on a stable
//! overlay — Leptos `For` + signals animated correctly but removed the node
//! after ~35ms in-browser, so DOM append/remove is intentional here.
//!
//! SSR / non-hydrate builds still render the empty overlay so markup matches.

use leptos::prelude::*;
#[cfg(feature = "hydrate")]
use uf_product::theme::PRODUCT_BRAND_PRESETS;

/// Fully visible hold before fade (~75% of total).
#[cfg(feature = "hydrate")]
const LINGER_MS: u64 = 1_200;
#[cfg(feature = "hydrate")]
const FADE_MS: u64 = 500;
#[cfg(feature = "hydrate")]
const TOTAL_MS: u64 = LINGER_MS + FADE_MS;

const FLOAT_KEYFRAMES: &str = r#"
@keyframes uf-counter-delta-float {
  0% {
    opacity: 1;
    transform: translate(-50%, -50%) translateY(0) rotate(var(--floater-rot, 0deg));
  }
  75% {
    opacity: 1;
    transform: translate(-50%, -50%) translateY(-20px) rotate(var(--floater-rot, 0deg));
  }
  100% {
    opacity: 0;
    transform: translate(-50%, -50%) translateY(-80px) rotate(var(--floater-rot, 0deg));
  }
}
.uf-counter-delta-floater {
  position: absolute;
  font-weight: 700;
  line-height: 1;
  opacity: 1;
  transform: translate(-50%, -50%);
  pointer-events: none;
  will-change: transform, opacity;
  animation: uf-counter-delta-float 1700ms linear forwards;
}
"#;

#[cfg(feature = "hydrate")]
fn next_rand(state: &mut u64) -> f64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x as f64) / (u64::MAX as f64)
}

/// Non-deterministic seed so hard-refresh does not replay the same floater look.
#[cfg(feature = "hydrate")]
fn entropy_seed() -> u64 {
    let millis = js_sys::Date::now().to_bits();
    let perf = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| (p.now() as f64).to_bits())
        .unwrap_or(0);
    let mut seed = millis ^ perf.rotate_left(17) ^ 0xc0ff_ee5e_ed00_u64;
    if seed == 0 {
        seed = 0xDEAD_BEEF_CAFE_BABE;
    }
    seed
}

/// Mix wall-clock entropy into the PRNG before each spawn.
#[cfg(feature = "hydrate")]
fn scramble_rng(state: &mut u64) {
    *state ^= entropy_seed().wrapping_mul(0x9E37_79B9_7F4A_7C15);
    if *state == 0 {
        *state = 1;
    }
    // Advance once so the first drawn value is not correlated with the raw seed.
    let _ = next_rand(state);
}

#[cfg(feature = "hydrate")]
fn spawn_dom_floater(overlay: &web_sys::Element, amount: usize, rng: &mut u64) {
    let Some(document) = overlay.owner_document() else {
        return;
    };
    let Ok(el) = document.create_element("span") else {
        return;
    };
    let preset_idx = (next_rand(rng) * PRODUCT_BRAND_PRESETS.len() as f64) as usize
        % PRODUCT_BRAND_PRESETS.len();
    let color = PRODUCT_BRAND_PRESETS[preset_idx].1;
    // Random tilt in both directions within ±15°.
    let rotation_deg = -15.0 + next_rand(rng) * 30.0;
    let font_size_px = 18.0 + next_rand(rng) * 22.0;

    // Scatter around the global count label when present; otherwise stage center.
    let (left_px, top_px) = floater_origin_px(overlay, rng);

    el.set_class_name("uf-counter-delta-floater");
    el.set_text_content(Some(&format!("+{amount}")));
    // Stable hooks for Playwright: assert `+N` text and that origins/rotation vary.
    let _ = el.set_attribute("data-testid", "delta-floater");
    let _ = el.set_attribute("data-amount", &amount.to_string());
    let _ = el.set_attribute("data-left", &format!("{left_px:.1}"));
    let _ = el.set_attribute("data-top", &format!("{top_px:.1}"));
    let _ = el.set_attribute("data-rotation", &format!("{rotation_deg:.1}"));
    let _ = el.set_attribute(
        "style",
        &format!(
            "left: {left_px:.1}px; top: {top_px:.1}px; color: {color}; font-size: {font_size_px}px; --floater-rot: {rotation_deg:.1}deg;"
        ),
    );
    let _ = overlay.append_child(&el);

    // Remove the concrete node after the animation — do not go through Leptos signals.
    let el_to_remove = el.clone();
    let _ = leptos::leptos_dom::helpers::set_timeout_with_handle(
        move || {
            if let Some(parent) = el_to_remove.parent_node() {
                let _ = parent.remove_child(&el_to_remove);
            }
        },
        std::time::Duration::from_millis(TOTAL_MS + 50),
    );
}

/// Pixel origin inside the overlay, jittered around the global count metric.
#[cfg(feature = "hydrate")]
fn floater_origin_px(overlay: &web_sys::Element, rng: &mut u64) -> (f64, f64) {
    let overlay_rect = overlay.get_bounding_client_rect();
    let stage = overlay.parent_element();
    let anchor = stage.as_ref().and_then(|p| {
        p.query_selector("[data-testid='global-counter']")
            .ok()
            .flatten()
    });

    let (base_x, base_y) = if let Some(anchor) = anchor {
        let r = anchor.get_bounding_client_rect();
        (
            r.left() - overlay_rect.left() + r.width() * 0.5,
            r.top() - overlay_rect.top() + r.height() * 0.5,
        )
    } else {
        (overlay_rect.width() * 0.5, overlay_rect.height() * 0.55)
    };

    // Random position in a loose cluster around the count (±~56px / ±~28px).
    let left_px = base_x + (next_rand(rng) - 0.5) * 112.0;
    let top_px = base_y + (next_rand(rng) - 0.5) * 56.0;
    (left_px, top_px)
}

/// Overlay that shows floating `+N` labels when the global counter increases.
///
/// Diffs `confirmed_global` across effects. Positive deltas spawn one floater
/// (hydrate only) colored from UF brand seeds. Keep this component mounted
/// across Photon WS refetches so remote increments still animate.
#[component]
pub fn DeltaFloaters(confirmed_global: Signal<usize>) -> impl IntoView {
    let overlay_ref = NodeRef::<leptos::html::Div>::new();
    let last_seen = StoredValue::new(None::<usize>);
    // 0 = uninitialized; first spawn seeds from wall-clock entropy.
    let rng = StoredValue::new(0_u64);

    Effect::new(move |_| {
        let new_value = confirmed_global.get();
        let prev = last_seen.get_value();
        last_seen.set_value(Some(new_value));
        let Some(prev) = prev else {
            return;
        };
        if new_value <= prev {
            return;
        }
        let delta = new_value - prev;

        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            let Some(overlay) = overlay_ref.get() else {
                return;
            };
            let Ok(overlay) = overlay.dyn_into::<web_sys::Element>() else {
                return;
            };
            let mut state = rng.get_value();
            scramble_rng(&mut state);
            spawn_dom_floater(&overlay, delta, &mut state);
            rng.set_value(state);
        }

        #[cfg(not(feature = "hydrate"))]
        {
            let _ = (delta, overlay_ref, rng);
        }
    });

    view! {
        <style>{FLOAT_KEYFRAMES}</style>
        // G7: absolute overlay for imperative DOM floaters; Orbital Box has no inset/absolute props.
        <div
            node_ref=overlay_ref
            data-testid="delta-floaters"
            style="position: absolute; inset: 0; pointer-events: none; overflow: visible; z-index: 2;"
        />
    }
}
