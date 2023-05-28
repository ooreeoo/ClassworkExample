
use std::io::Read;
use serde::Deserialize;

static DATA_URL: &'static str = "https://2a2d0e3f-9b06-4381-a183-f2a75519cadf.mysimplestore.com/api/v2/products/asi-bac4000-plug-and-play-kit-for-surron";
static EXPECTED_RESPONSE: ResponsePayload<'static> = ResponsePayload {
    updated_at: "2021-02-10T02:09:37.000Z",
    total_on_hand: 0,
    master: MasterPayload {
        in_stock: false,
        total_on_hand: 0
    }
};

#[derive(Eq, PartialEq, Deserialize, Debug)]
struct ResponsePayload<'data> {
    #[serde(borrow)]
    updated_at: &'data str,
    total_on_hand: i64,
    master: MasterPayload
}

#[derive(Eq, PartialEq, Deserialize, Debug)]
struct MasterPayload {
    in_stock: bool,
    total_on_hand: i64
}

struct NotificationNeeded {
    content: String,
    fatal: bool
}

fn main() {
    let twilio = Twilio::from_env();
    let mut limiter = ratelimit::Builder::new()
        .capacity(1)
        .quantum(1)
        .interval(std::time::Duration::from_secs(20))
        .build();
    loop {
        limiter.wait();
        if let Err(notification) = check_stock() {
            let fatal = notification.fatal;
            if twilio.send(notification).is_ok() && fatal {
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(60 * 2));
        }
}
}

fn check_stock() -> Result<(), NotificationNeeded> {
    let response = match ureq::get(DATA_URL)
        .set("User-Agent", "EMotoBros Stock Checking Bot, 3 req/minute, contact ekardnt@ekardnt.com for problems, see https://github.com/EkardNT/bac-bot")
        .call() {
        Ok(response) => {
            if response.status() == 200 {
                response
            } else {
                return Err(NotificationNeeded {
                    content: format!("Unexpected successful status code {}", response.status()),
                    fatal: true
                });
            }
        },
        Err(ureq::Error::Status(code, _response)) => {
            return Err(NotificationNeeded {
                content: format!("Status code {}", code),
                fatal: code >= 400 && code < 500
            });
        },
        Err(ureq::Error::Transport(err)) => {
            return Err(NotificationNeeded {
                content: format!("Transport error retrieving JSON data: {:?}", err),
                fatal: false