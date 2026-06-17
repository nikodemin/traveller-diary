use self::super::dao::Dao;
use crate::dao::DaoOps;
use crate::model::{Cmd, Response};
use log::{Level, log};
use std::sync::mpsc::{Receiver, Sender};

pub struct Backend<Dao: DaoOps> {
    dao: Dao,
    rsp_sender: Sender<Response>,
    cmd_receiver: Receiver<Cmd>,
}

impl<Dao: DaoOps> Backend<Dao> {
    pub fn new(dao: Dao, rsp_sender: Sender<Response>, cmd_receiver: Receiver<Cmd>) -> Self {
        Backend {
            dao,
            rsp_sender,
            cmd_receiver,
        }
    }

    pub fn serve(&self) {
        while let Ok(cmd) = self.cmd_receiver.recv() {
            match cmd {
                Cmd::LoadTravels { limit, page } => match self.dao.list_travels(limit, page) {
                    Ok(travels) => self
                        .rsp_sender
                        .send(Response::LoadTravels { page, travels })
                        .unwrap(),
                    Err(err) => log!(Level::Error, "failed to list travels, err: {}", err),
                },
            }
        }
        log!(Level::Error, "Serve loop exit, channel disconnected.");
    }
}
