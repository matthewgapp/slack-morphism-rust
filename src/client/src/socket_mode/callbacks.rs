use crate::errors::*;
use crate::listener::SlackClientEventsUserStateStorage;
use crate::prelude::UserCallbackFunction;
use crate::{ClientResult, SlackClient, SlackClientHttpConnector};
use futures::future::BoxFuture;
use log::*;
use slack_morphism_models::events::{SlackCommandEvent, SlackCommandEventResponse};
use slack_morphism_models::socket_mode::SlackSocketModeHelloEvent;
use std::future::Future;
use std::sync::{Arc, RwLock};

pub trait SlackSocketModeListenerCallback<SCHC, RQ, RS>
where
    SCHC: SlackClientHttpConnector + Send + Sync + 'static,
    RQ: Send + Sync + 'static,
    RS: Send + Sync + 'static,
{
    fn call(
        &self,
        ev: RQ,
        client: Arc<SlackClient<SCHC>>,
        state_storage: Arc<RwLock<SlackClientEventsUserStateStorage>>,
    ) -> BoxFuture<'static, RS>;
}

impl<T, F, SCHC, RQ, RS> SlackSocketModeListenerCallback<SCHC, RQ, RS> for T
where
    T: Send
        + Sync
        + Fn(RQ, Arc<SlackClient<SCHC>>, Arc<RwLock<SlackClientEventsUserStateStorage>>) -> F,
    F: Future<Output = RS> + Send + Sync + 'static,
    SCHC: SlackClientHttpConnector + Send + Sync + 'static,
    RQ: Send + Sync + 'static,
    RS: Send + Sync + 'static,
{
    fn call(
        &self,
        ev: RQ,
        client: Arc<SlackClient<SCHC>>,
        state_storage: Arc<RwLock<SlackClientEventsUserStateStorage>>,
    ) -> BoxFuture<'static, RS> {
        Box::pin(self(ev, client, state_storage))
    }
}

pub struct SlackSocketModeListenerCallbacks<SCHC>
where
    SCHC: SlackClientHttpConnector + Send + Sync + 'static,
{
    pub hello_callback:
        Box<dyn SlackSocketModeListenerCallback<SCHC, SlackSocketModeHelloEvent, ()> + Send + Sync>,
    pub command_callback: Box<
        dyn SlackSocketModeListenerCallback<
                SCHC,
                SlackCommandEvent,
                ClientResult<SlackCommandEventResponse>,
            > + Send
            + Sync,
    >,
}

impl<SCHC> SlackSocketModeListenerCallbacks<SCHC>
where
    SCHC: SlackClientHttpConnector + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            hello_callback: Box::new(Self::empty_hello_callback),
            command_callback: Box::new(Self::empty_command_events_callback),
        }
    }

    pub fn with_command_events<F>(
        mut self,
        command_events_fn: UserCallbackFunction<SlackCommandEvent, F, SCHC>,
    ) -> Self
    where
        F: Future<Output = ClientResult<SlackCommandEventResponse>> + Send + Sync + 'static,
    {
        self.command_callback = Box::new(command_events_fn);
        self
    }

    async fn empty_hello_callback(
        event: SlackSocketModeHelloEvent,
        _client: Arc<SlackClient<SCHC>>,
        _states: Arc<RwLock<SlackClientEventsUserStateStorage>>,
    ) {
        debug!("Received Slack hello for socket mode: {:?}", event);
    }

    async fn empty_command_events_callback(
        event: SlackCommandEvent,
        _client: Arc<SlackClient<SCHC>>,
        _states: Arc<RwLock<SlackClientEventsUserStateStorage>>,
    ) -> Result<SlackCommandEventResponse, Box<dyn std::error::Error + Send + Sync>> {
        warn!("No callback is specified for a command event: {:?}", event);
        Err(Box::new(SlackClientError::SystemError(
            SlackClientSystemError::new("No callback is specified for a command event".to_string()),
        )))
    }
}
