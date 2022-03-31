use crossbeam::channel::{Receiver, RecvError, Sender, TryRecvError, TrySendError};

/// Create a pair of exchangers which we can use to send and receive large allocated collections (Boxed arrays, Vecs, etc)
pub fn exchangers<T: Clone>(first_item: T) -> (ExchangerSender<T>, ExchangerReceiver<T>) {
    let (sender, receiver) = crossbeam::channel::bounded(1);
    let (buffer_sender, buffer_receiver) = crossbeam::channel::bounded(1);
    // Fill the buffer channel to start the transactions.
    let _ = buffer_sender.send(first_item.clone());

    let mut exc_recv = ExchangerReceiver {
        item_receiver: receiver,
        buffer_sender,
        last_item: first_item,
    };

    let exc_send = ExchangerSender {
        item_sender: sender,
        buffer_receiver,
    };

    (exc_send, exc_recv)
}

pub struct ExchangerReceiver<T> {
    item_receiver: Receiver<T>,
    buffer_sender: Sender<T>,
    last_item: T,
}

impl<T> ExchangerReceiver<T> {
    pub fn recv(&mut self) -> Result<&mut T, crossbeam::channel::RecvError> {
        let frame = self.item_receiver.recv()?;
        let old_frame = std::mem::replace(&mut self.last_item, frame);
        // Send the old frame back to the initial sender
        self.buffer_sender.send(old_frame).map_err(|_| RecvError)?;

        Ok(&mut self.last_item)
    }

    pub fn try_recv(&mut self) -> Result<&mut T, crossbeam::channel::TryRecvError> {
        let frame = self.item_receiver.try_recv()?;
        let old_frame = std::mem::replace(&mut self.last_item, frame);
        // Send the old frame back to the initial sender
        self.buffer_sender
            .send(old_frame)
            .map_err(|_| TryRecvError::Disconnected)?;

        Ok(&mut self.last_item)
    }

    pub fn most_recent_received(&self) -> &T {
        &self.last_item
    }

    pub fn try_recv_or_recent(&mut self) -> &T {
        let _ = self.try_recv();
        &self.last_item
    }
}

pub struct ExchangerSender<T> {
    item_sender: Sender<T>,
    buffer_receiver: Receiver<T>,
}

impl<T> ExchangerSender<T> {
    pub fn send(&self, frame: &mut T) -> Result<(), crossbeam::channel::SendError<()>> {
        let old_buffer = self
            .buffer_receiver
            .recv()
            .map_err(|_| crossbeam::channel::SendError(()))?;
        let new_frame = std::mem::replace(frame, old_buffer);
        self.item_sender
            .send(new_frame)
            .map_err(|_| crossbeam::channel::SendError(()))?;

        Ok(())
    }

    pub fn try_send(&self, frame: &mut T) -> Result<(), crossbeam::channel::TrySendError<()>> {
        let old_buffer = self
            .buffer_receiver
            .recv()
            .map_err(|e| crossbeam::channel::TrySendError::Disconnected(()))?;
        let new_frame = std::mem::replace(frame, old_buffer);

        self.item_sender.try_send(new_frame).map_err(|e| match e {
            TrySendError::Full(_) => TrySendError::Full(()),
            TrySendError::Disconnected(_) => TrySendError::Disconnected(()),
        })?;

        Ok(())
    }
}
