use codee::string::FromToStringCodec;
use leptos::prelude::*;
use leptos_use::storage::*;

/// A parameterized incrementing button
#[component]
pub fn Button(#[prop(default = 1)] increment: i32) -> impl IntoView {
    // let (count, set_count) = signal(0);
    let (count, set_count, _) = use_local_storage::<i32, FromToStringCodec>("cnt");
    view! {
        <button on:click=move |_| {
            set_count.set(count.get() + increment)
        }>

            "Add " {increment} ": " {count}
        </button>
    }
}
