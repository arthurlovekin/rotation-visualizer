//! Collapsible section with a dropdown button next to the title.

use leptos::prelude::*;

/// A section with a title and dropdown button that toggles visibility of its content.
#[component]
pub fn CollapsibleSection(
    /// Section title (e.g. "Axis-Angle", "Quaternion").
    title: &'static str,
    /// Content to show/hide. Wrapped in a div when visible.
    children: Children,
) -> impl IntoView {
    let expanded = RwSignal::new(true);

    view! {
        <div class="control-section">
            <div class="section-header">
                <button
                    type="button"
                    class="dropdown-button"
                    aria-expanded=move || expanded.get()
                    aria-label=move || if expanded.get() { "Collapse section" } else { "Expand section" }
                    on:click=move |_| expanded.update(|v| *v = !*v)
                >
                    <span class="dropdown-icon" class:collapsed=move || !expanded.get()>
                        "▼"
                    </span>
                </button>
                <h2 class="section-title">{title}</h2>
            </div>
            <div class="section-content" class:section-collapsed=move || !expanded.get()>
                {children()}
            </div>
        </div>
    }
}
