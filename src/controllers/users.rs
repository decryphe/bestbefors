use loco_rs::prelude::*;

#[debug_handler]
pub async fn login_page(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "auth/login.html", data!({}))
}

#[debug_handler]
pub async fn register_page(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "auth/register.html", data!({}))
}

#[debug_handler]
pub async fn logout_page(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "auth/logout.html", data!({}))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("auth")
        .add("/login", get(login_page))
        .add("/register", get(register_page))
        .add("/logout", get(logout_page))
}
