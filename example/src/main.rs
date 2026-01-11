use rust_i18n_autotranslate::translate;

#[tokio::main]
async fn main() {
    let res = translate("./locales", "en", ["fr"].to_vec(), true).await;

    println!("{res:?}")
}
