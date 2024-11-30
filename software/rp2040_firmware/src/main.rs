#![no_std]
#![no_main]

use hackernewyears::menu::MenuBinding;
use hackernewyears::AnimatingGif;
use hackernewyears::AnimatingGifs;

use defmt_rtt as _;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut devices = hackernewyears::Devices::new(p);
    let animating_gifs = AnimatingGifs::new();

    for _ in 0..5 {
        animating_gifs
            .animate(AnimatingGif::Logo, &mut devices)
            .await;
    }

    loop {
        #[derive(Clone)]
        pub enum MainMenuResult {
            UpMenu,
            Eyes,
            Abstract,
        }

        let result = hackernewyears::menu::run_menu::<MainMenuResult>(
            &[
                MenuBinding::new("Main Menu", None),
                MenuBinding::new("Eyes Animated Gif", Some(MainMenuResult::Eyes)),
                MenuBinding::new("Abstract", Some(MainMenuResult::Abstract)),
            ],
            MainMenuResult::UpMenu,
            &mut devices,
        )
        .await;

        match result {
            MainMenuResult::UpMenu => {} // Already at the top
            MainMenuResult::Eyes => {
                animating_gifs
                    .animate(AnimatingGif::Eyes, &mut devices)
                    .await
            }
            MainMenuResult::Abstract => {
                animating_gifs
                    .animate(AnimatingGif::Abstract, &mut devices)
                    .await
            }
        }
    }
}
