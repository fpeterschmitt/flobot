use crate::client;
use crate::db;
use crate::db::tempo::Tempo;
use crate::handlers::{Handler, Result};
use crate::models::GenericPost;
use crate::models::Trigger as MTrigger;
use regex::escape as escape_re;
use regex::Regex;
use std::rc::Rc;
use std::time::Duration;

pub fn compile_trigger(trigger: &str) -> std::result::Result<Regex, regex::Error> {
    let re = format!("(?ms)^.*({}).*$", escape_re(trigger));
    Regex::new(&re)
}

pub fn valid_match(find: &str, message: &str) -> bool {
    let captured = message.find(find);
    if captured.is_none() {
        return false;
    }

    let start = captured.unwrap();
    let end = start + find.len() - 1;

    if start > 0 {
        if !message.as_bytes()[start - 1].is_ascii_whitespace() {
            return false;
        }
    }

    if end + 1 < message.len() {
        if !message.as_bytes()[end + 1].is_ascii_whitespace() {
            return false;
        }
    }

    true
}

pub struct Trigger<C, E> {
    db: Rc<E>,
    client: Rc<C>,
    match_list: Regex,
    match_del: Regex,
    match_text: Regex,
    match_reaction: Regex,
    tempo: Tempo<String>,
    delay_repeat: Duration,
}

impl<C, E> Trigger<C, E> {
    pub fn new(db: Rc<E>, client: Rc<C>, tempo: Tempo<String>, delay_repeat: Duration) -> Self {
        Self {
            db,
            client,
            tempo,
            delay_repeat,
            match_list: Regex::new("^!trigger list.*$").unwrap(),
            match_del: Regex::new("^!trigger del \"(.+)\".*").unwrap(),
            match_reaction: Regex::new("^!trigger reaction \"([^\"]+)\" [:\"]([^:]+)[:\"].*$").unwrap(),
            match_text: Regex::new("^!trigger text \"([^\"]+)\" \"([^\"]+)\".*$").unwrap(),
        }
    }

    pub fn match_trigger(&self, message: &str, trigger: &String) -> bool {
        return valid_match(trigger, message);
    }
}

impl<C, E> Handler for Trigger<C, E>
where
    C: client::Sender,
    E: db::Trigger,
{
    type Data = GenericPost;

    fn name(&self) -> &str {
        "trigger"
    }

    fn help(&self) -> Option<String> {
        Some(format!(
            "```
Automatically react to a given text in each received message on channels where the bot is present.

There is a per channel antispam of 3 seconds, avoiding a heated channel to be polluted by the bot.

A per [channel, trigger] antispam is effective and currently configured at {} seconds.

!trigger list
!trigger text \"trigger\" \"me\"
!trigger reaction \"trigger\" :emoji:
!trigger del \"trigger\"
```",
            self.delay_repeat.as_secs()
        ))
    }

    fn handle(&self, post: &GenericPost) -> Result {
        let message = post.message.as_str();

        if !message.starts_with("!trigger ") {
            // check or set a per channel rate limit to avoid spamming in heated discussions.
            let tempo_rate = format!("{}{}--global-channel-rate-limit", &post.team_id, &post.channel_id);
            if self.tempo.exists(&tempo_rate) {
                return Ok(());
            }
            self.tempo.set(tempo_rate.clone(), Duration::from_secs(3));

            // search for triggers in the message
            let team_triggers = self.db.search(&post.team_id)?;
            for t in team_triggers
                .iter()
                .filter(|tt| self.match_trigger(&post.message, &tt.triggered_by))
                .collect::<Vec<&MTrigger>>()
            {
                let tempo_key = format!(
                    "{}{}{}--trigger-channel-rate-limit",
                    &post.team_id, &post.channel_id, t.triggered_by
                );

                // sending this trigger has been delayed
                if self.tempo.exists(&tempo_key) {
                    continue;
                }
                self.tempo.set(tempo_key.clone(), self.delay_repeat);

                if t.text_.is_some() {
                    // text is sorted after emoji, so we can break here: emoji were already processed.
                    self.client.reply(post, t.text_.as_ref().unwrap())?;
                    break;
                } else {
                    // send all emoji reactions
                    self.client.reaction(post, &t.emoji.as_ref().unwrap())?;
                }
            }
            return Ok(());
        }

        if self.match_list.is_match(message) {
            let res = self.db.list(&post.team_id)?;
            return Ok(self.client.send_trigger_list(res, post)?);
        }

        match self.match_text.captures(message) {
            Some(captures) => {
                let trigger = captures.get(1).unwrap().as_str();

                // prevent insertion of broken triggers.
                if let Err(e) = compile_trigger(trigger) {
                    return Ok(self.client.reply(post, &e.to_string())?);
                }

                let _ = self.db.add_text(&post.team_id, trigger, captures.get(2).unwrap().as_str());
                return Ok(self.client.reaction(post, "ok_hand")?);
            }
            None => {}
        }

        match self.match_reaction.captures(message) {
            Some(captures) => {
                let trigger = captures.get(1).unwrap().as_str();

                // prevent insertion of broken triggers.
                if let Err(e) = compile_trigger(trigger) {
                    return Ok(self.client.reply(post, &e.to_string())?);
                }

                let _ = self.db.add_emoji(&post.team_id, trigger, captures.get(2).unwrap().as_str());
                return Ok(self.client.reaction(post, "ok_hand")?);
            }
            None => {}
        }

        match self.match_del.captures(message) {
            Some(captures) => {
                let _ = self.db.del(&post.team_id, captures.get(1).unwrap().as_str())?;
                return Ok(self.client.reaction(post, "ok_hand")?);
            }
            None => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm(message: &str) -> bool {
        valid_match("trig", message)
    }

    #[test]
    fn test_valid_match_left() {
        assert!(vm("trig "));
        assert!(vm("trig yes"));
        assert!(!vm("trigno"));
    }

    #[test]
    fn test_valid_match_end() {
        assert!(vm(" trig"));
        assert!(vm("ye trig"));
        assert!(!vm("notrig"));
    }

    #[test]
    fn test_valid_match_between() {
        assert!(vm("trig"));
        assert!(vm(" trig "));
        assert!(vm("yes trig yes"));
        assert!(!vm("notrigno"));
    }

    #[test]
    fn test_valid_match_nbsp() {
        assert!(!vm("no\u{A0}trig\u{A0}no"));
    }
}
