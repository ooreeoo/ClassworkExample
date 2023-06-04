
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
            });
        }
    };

    let content_length = response.header("Content-Length")
        .map(|header| header.parse::<usize>())
        .transpose()
        .map_err(|parse_err| NotificationNeeded {
            content: format!("Content-Length present but not a valid usize: {:?}", parse_err),
            fatal: true
        })?
        .unwrap_or(8192);

    let mut bytes = Vec::with_capacity(content_length);
    response.into_reader().read_to_end(&mut bytes).map_err(|io_err| NotificationNeeded {
        content: format!("IO error when reading HTTP body: {:?}", io_err),
        fatal: false
    })?;

    let payload: ResponsePayload<'_> = serde_json::from_slice(bytes.as_slice())
        .map_err(|deser_err| NotificationNeeded {
            content: format!("JSON deserialization failed: {:?}", deser_err),
            fatal: true
        })?;


    if payload == EXPECTED_RESPONSE {
        eprintln!("Response is as expected");
        return Ok(());
    }

    if payload.total_on_hand > 0 || payload.master.total_on_hand > 0 || payload.master.in_stock {
        return Err(NotificationNeeded {
            content: format!("BAC4000 maybe in stock! {:#?}", payload),
            fatal: true
        });
    }

    return Err(NotificationNeeded {
        content: format!("Payload updated but looks like still not in stock, update the expected payload to {:#?}", payload),
        fatal: true
    });
}

struct Twilio {
    twilio_sid: String,
    twilio_auth_token: String,
    twilio_source_phone: String,
    twilio_destination_phone: String
}

impl Twilio {
    fn from_env() -> Self {
        let twilio_sid = std::env::var("TWILIO_SID")
        .expect("TWILIO_SID environment variable not found");
        let twilio_auth_token = std::env::var("TWILIO_AUTH_TOKEN")
            .expect("TWILIO_AUTH_TOKEN environment variable not found");
        let twilio_source_phone = std::env::var("TWILIO_SOURCE_PHONE")
            .expect("TWILIO_SOURCE_PHONE environment variable not found");
        let twilio_destination_phone = std::env::var("TWILIO_DESTINATION_PHONE")
            .expect("TWILIO_DESTINATION_PHONE environment variable not found");
        Self {
            twilio_sid,
            twilio_auth_token,
            twilio_source_phone,
            twilio_destination_phone
        }
    }

    fn send(&self, notification: NotificationNeeded) -> Result<(), ()> {
        let mut form_params: Vec<(&str, &str)> = Vec::with_capacity(3);
        form_params.push(("Body", &notification.content));
        form_params.push(("From", &self.twilio_source_phone));
        form_params.push(("To", &self.twilio_destination_phone));
        let url = format!("https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json", self.twilio_sid);
        let auth = base64::encode(format!("{}:{}", self.twilio_sid, self.twilio_auth_token));
        ureq::post(&url)
            .set("Authorization", &format!("Basic {}", auth))
            .send_form(form_params.as_slice())
            .map(|_response| {
                eprintln!("HTTP call to Twilio API succeeded, notification: {}", notification.content);
                ()
            })
            .map_err(|err| {
                eprintln!("HTTP call to Twilio API failed: {:?}", err);
                ()
            })
    }
}