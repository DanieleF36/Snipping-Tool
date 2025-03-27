use global_hotkey::GlobalHotKeyEvent;
use std::future;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};

pub struct Hotkey<Message> {
    pub msg: Message,
}

impl<Message: Clone> future::Future for Hotkey<Message> {
    type Output = (Message, ());

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        static WAKER: Mutex<Option<std::task::Waker>> = Mutex::new(None);
        static FIRED_HOTKEY: Mutex<Option<u32>> = Mutex::new(None);

        // Questa funzione, internamente, salva la il callback in una
        // "OnceCell", struttura dati alla quale si può assegnare una volta
        // sola. È quindi ok chiamarla più volte.
        GlobalHotKeyEvent::set_event_handler(Some(|ev: GlobalHotKeyEvent| {
            if let Ok(w_mg) = WAKER.lock() {
                if let Some(w) = &*w_mg {
                    w.clone().wake();

                    let mut fh_mg = FIRED_HOTKEY.lock().unwrap();
                    *fh_mg = Some(ev.id);
                }
            }
        }));

        if let Ok(mut w_mg) = WAKER.lock() {
            *w_mg = Some(cx.waker().clone());
        } else {
            return Poll::Pending;
        };

        if let Ok(mut fh_mg) = FIRED_HOTKEY.lock() {
            if fh_mg.is_some() {
                *fh_mg = None;
                return Poll::Ready((self.msg.clone(), ()));
            }
        }

        return Poll::Pending;
    }
}
