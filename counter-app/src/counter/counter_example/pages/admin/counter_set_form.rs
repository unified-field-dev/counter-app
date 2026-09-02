//! Orbital form that dispatches Higgs `SetCounter` for the admin page.

use crate::counter::counter_example::server::SetCounter;
use leptos::form::ActionForm;
use leptos::prelude::*;
use orbital::components::SpacingSize;
use orbital::primitives::{
    Button, ButtonType, Field, Flex, Input, InputAppearance, InputBind, InputType,
};

/// Form for setting the counter to a specific value.
///
/// Bind a `ServerAction<SetCounter>` from [`super::CounterAdminPage`]. The
/// Leptos `ActionForm` posts the `value` field to the Higgs server fn; Orbital
/// `Input` / `Button` provide the chrome. On success the parent syncs from the
/// action result and Photon publish (inside `counter_set`) updates live clients.
#[component]
pub fn CounterSetForm(set_action: ServerAction<SetCounter>) -> impl IntoView {
    view! {
        <ActionForm action=set_action>
            <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                <Field label="Counter value">
                    <div data-testid="set-input">
                        <Input
                            bind=InputBind { name: "value".into(), ..InputBind::default() }
                            appearance=InputAppearance {
                                input_type: Signal::from(InputType::Number),
                                placeholder: "Enter a new counter value".into(),
                                ..Default::default()
                            }
                        />
                    </div>
                </Field>
                <div data-testid="set-submit">
                    <Button button_type=ButtonType::Submit>
                        "Set Counter"
                    </Button>
                </div>
            </Flex>
        </ActionForm>
    }
}
