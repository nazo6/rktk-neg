use rktk::config::keymap::{
    key_manager::keycode::{
        key::*, layer::*, media::*, modifier::*, mouse::*, special::*, utils::*, *,
    },
    Keymap, Layer, LayerKeymap,
};
use rktk_keymanager::keymap::TapDanceDefinition;

const L2ENTER: KeyAction = KeyAction::TapHold(
    KeyCode::Key(Key::Enter),
    KeyCode::Layer(LayerOp::Momentary(2)),
);

const L2SPC: KeyAction = KeyAction::TapHold(
    KeyCode::Key(Key::Enter),
    KeyCode::Layer(LayerOp::Momentary(2)),
);

const L3SPC: KeyAction = KeyAction::TapHold(
    KeyCode::Key(Key::Enter),
    KeyCode::Layer(LayerOp::Momentary(3)),
);

const L4GRV: KeyAction = KeyAction::TapHold(
    KeyCode::Key(Key::Grave),
    KeyCode::Layer(LayerOp::Momentary(4)),
);

#[rustfmt::skip]
const L0: LayerKeymap = [
    [ L4GRV , D1    , D2    , D3    , D4    , D5    , _____ , _____ , /**/ _____ , _____ , D6    , D7    , D8    , D9    , D0    , EQUAL ],
    [  TAB  , Q     , W     , E     , R     , T     , _____ , _____ , /**/ _____ , _____ , Y     , U     , I     , O     , P     , MINUS ],
    [  ESC  , A     , S     , D     , F     , G     , _____ , _____ , /**/ _____ , _____ , H     , J     , K     , L     , SCLN  , QUOTE ],
    [ L_SHFT, Z     , X     , C     , V     , B     , LBRC  , _____ , /**/ _____ , TD(0) , N     , M     , COMM  , DOT   , SLASH , BSLSH ],
    [ L_CTRL, L_GUI , TG(2) , L_ALT , L3SPC , L2SPC , SPACE , _____ , /**/ BS    , BS    ,L2ENTER, _____ , _____ , _____ , R_SHFT, R_CTRL],
];

#[rustfmt::skip]
/// Auto mouse layer
const L1: LayerKeymap = [
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____,AML_RESET,M_LEFT,MO_SCRL,M_RIGHT, _____ , _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ ,M_BACK,M_MIDDLE,M_FORWARD,_____, _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
];

#[rustfmt::skip]
/// Mouse layer
const L2: LayerKeymap = [
    [ _____ , F1    , F2    , F3    , F4    , F5    , _____ ,_____ , /**/ _____ ,_____ , F6    , F7    , F8    , TG(2) , F10   , F11   ],
    [ _____ , _____ , INSERT, HOME  , PGUP  , _____ , _____ ,_____ , /**/ _____ ,_____ , LEFT  , DOWN  , UP    , RIGHT , _____ , F12   ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____,AML_RESET,M_LEFT,MO_SCRL,M_RIGHT, _____ , VOLUP ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ ,M_BACK,M_MIDDLE,M_FORWARD,_____, VOLDN ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,DELETE, _____ , _____ , _____ , _____ , PRTSC , _____ ],
];

const FL_CLR: KeyAction = FLASH_CLEAR;
#[rustfmt::skip]
const L3: LayerKeymap = [
    [ FL_CLR, _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,__ ,OUTPUT_BLE,OUTPUT_USB , __, _____ , _____ , _____ ],
    [ _____ , _____ , KP7   , KP8   , KP9   , _____ , _____ ,_____ , /**/ _____ ,_____ , SF(D1), SF(D2), SF(D3), SF(D4), SF(D5), _____ ],
    [ _____ , _____ , KP4   , KP5   , KP6   , _____ , _____ ,_____ , /**/ _____ ,_____ , SF(D6), SF(D7), SF(D8), SF(D9), SF(D0), _____ ],
    [ _____ , _____ , KP1   , KP2   , KP3   , _____ , _____ ,_____ , /**/ _____ ,_____ , QUOTE,SF(QUOTE),EQUAL,SF(EQUAL), _____, _____ ],
    [ _____ , _____ , KP0   , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
];

#[rustfmt::skip]
const L4: LayerKeymap = [
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
    [ _____ , _____ , _____ , _____ , _____ , _____ , _____ ,_____ , /**/ _____ ,_____ , _____ , _____ , _____ , _____ , _____ , _____ ],
];

pub const KEYMAP: Keymap = Keymap {
    encoder_keys: [(
        KeyCode::Media(Media::VolumeIncrement),
        KeyCode::Media(Media::VolumeDecrement),
    )],
    layers: [
        Layer {
            keymap: L0,
            arrowmouse: false,
        },
        Layer {
            keymap: L1,
            arrowmouse: false,
        },
        Layer {
            keymap: L2,
            arrowmouse: false,
        },
        Layer {
            keymap: L3,
            arrowmouse: true,
        },
        Layer {
            keymap: L4,
            arrowmouse: true,
        },
    ],
    tap_dance: [Some(TapDanceDefinition {
        tap: [
            Some(KeyCode::Key(Key::RightBracket)),
            Some(KeyCode::Layer(LayerOp::Toggle(2))),
            Some(KeyCode::Layer(LayerOp::Toggle(3))),
            Some(KeyCode::Layer(LayerOp::Toggle(4))),
        ],
        hold: [None, None, None, None],
    })],
    combo: [],
};
