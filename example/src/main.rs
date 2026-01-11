use rust_i18n_autotranslate::translate;

fn main() {
    let res = translate("./locales", "en", ["fr"].to_vec(), true);
    println!("{res:?}")
}
