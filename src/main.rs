
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