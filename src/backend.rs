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
                Cmd::LoadTravelsByYear { year } => match self.dao.list_travels_by_year(year) {
                    Ok(travels) => self
                        .rsp_sender
                        .send(Response::LoadTravelsByYear { year, travels })
                        .unwrap(),
                    Err(err) => log!(Level::Error, "failed to list travels, err: {}", err),
                },
                Cmd::AddTravel { travel } => match self.dao.add_travel(travel) {
                    Ok(id) => self.rsp_sender.send(Response::AddTravel { id }).unwrap(),
                    Err(err) => log!(Level::Error, "failed to add travel, err: {}", err),
                },
            }
        }
        log!(Level::Error, "Serve loop exit, channel disconnected.");
    }
}
