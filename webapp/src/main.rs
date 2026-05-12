use yew::prelude::*;

mod components {
    pub mod home_page;
}
mod config;
mod models;

fn main() {
    yew::Renderer::<App>::new().render();
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <components::home_page::HomePage />
    }
}
