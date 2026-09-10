use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Sender},
    },
    time::Duration,
};

use wyn_protocol::{AgentCommand, AgentCommandKind, AgentResponse, PROTOCOL_VERSION};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct AgentSession {
    session_id: u64,
    sender: Sender<AgentCommand>,
}

struct PendingRequest {
    machine_id: String,
    sender: Sender<AgentResponse>,
}

struct ControlPlaneInner {
    agents: RwLock<HashMap<String, AgentSession>>,

    pending: Mutex<HashMap<u64, PendingRequest>>,

    next_request_id: AtomicU64,

    next_session_id: AtomicU64,
}

#[derive(Clone)]
pub struct ControlPlane {
    inner: Arc<ControlPlaneInner>,
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self {
            inner: Arc::new(ControlPlaneInner {
                agents: RwLock::new(HashMap::new()),

                pending: Mutex::new(HashMap::new()),

                next_request_id: AtomicU64::new(1),

                next_session_id: AtomicU64::new(1),
            }),
        }
    }
}

impl ControlPlane {
    pub fn register_agent(&self, machine_id: String, sender: Sender<AgentCommand>) -> u64 {
        let session_id = self.inner.next_session_id.fetch_add(1, Ordering::Relaxed);

        let mut agents = self
            .inner
            .agents
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        agents.insert(machine_id, AgentSession { session_id, sender });

        session_id
    }

    pub fn unregister_agent(&self, machine_id: &str, session_id: u64) {
        let mut agents = self
            .inner
            .agents
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let should_remove = agents
            .get(machine_id)
            .map(|session| session.session_id == session_id)
            .unwrap_or(false);

        if should_remove {
            agents.remove(machine_id);
        }
    }

    pub fn request(
        &self,
        machine_id: &str,
        command: AgentCommandKind,
    ) -> Result<AgentResponse, String> {
        let request_id = self.inner.next_request_id.fetch_add(1, Ordering::Relaxed);

        let agent_sender = {
            let agents = self
                .inner
                .agents
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            agents
                .get(machine_id)
                .map(|session| session.sender.clone())
                .ok_or_else(|| format!("machine `{machine_id}` has no active Agent session"))?
        };

        let (response_tx, response_rx) = mpsc::channel();

        {
            let mut pending = self
                .inner
                .pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            pending.insert(
                request_id,
                PendingRequest {
                    machine_id: machine_id.to_string(),

                    sender: response_tx,
                },
            );
        }

        let command = AgentCommand {
            protocol_version: PROTOCOL_VERSION,

            request_id,

            command,
        };

        if agent_sender.send(command).is_err() {
            self.remove_pending(request_id);

            return Err(format!("Agent `{machine_id}` command channel is closed"));
        }

        match response_rx.recv_timeout(REQUEST_TIMEOUT) {
            Ok(response) => Ok(response),

            Err(error) => {
                self.remove_pending(request_id);

                Err(format!("Agent request {request_id} failed: {error}"))
            }
        }
    }

    pub fn complete(&self, machine_id: &str, response: AgentResponse) -> bool {
        let pending_request = {
            let mut pending = self
                .inner
                .pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            let belongs_to_machine = pending
                .get(&response.request_id)
                .map(|request| request.machine_id == machine_id)
                .unwrap_or(false);

            if !belongs_to_machine {
                return false;
            }

            pending.remove(&response.request_id)
        };

        let Some(pending_request) = pending_request else {
            return false;
        };

        pending_request.sender.send(response).is_ok()
    }

    fn remove_pending(&self, request_id: u64) {
        self.inner
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&request_id);
    }
}
