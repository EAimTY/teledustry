use getopts::Options;

pub struct Config {
    pub token: String,
    pub proxy: Option<String>,
    pub webhook: u16,
    pub file: String,
}

impl Config {
    pub fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut opts = Options::new();

        opts.optopt(
            "t",
            "token",
            "(required) set Telegram Bot HTTP API token",
            "TOKEN",
        );
        opts.optopt(
            "p",
            "proxy",
            "set proxy (supported: http, https, socks5)",
            "PROXY",
        );
        opts.optopt(
            "w",
            "webhook-port",
            "set webhook port (1 ~ 65535) and run bot in webhook mode",
            "WEBHOOK_PORT",
        );
        opts.optflag("h", "help", "print this help menu");

        let usage = opts.usage(&format!("Usage: {} [options] FILE", args[0]));

        let matches = opts
            .parse(&args[1..])
            .or_else(|e| return Err(e.to_string()))?;

        let file = if matches.free.len() == 1 {
            matches.free[0].clone()
        } else {
            if matches.free.is_empty() {
                return Err(String::from("Server file not provided"));
            } else {
                let mut free = String::new();
                matches.free[1..]
                    .iter()
                    .for_each(|arg| free.push_str(&format!(" \"{}\"", arg)));

                return Err(format!("Unrecognized argument:{}", free));
            }
        };

        if matches.opt_present("h") {
            return Err(usage);
        }

        let token = matches
            .opt_str("t")
            .ok_or_else(|| String::from("Telegram Bot HTTP API token not set"))?;

        let proxy = matches.opt_str("p");

        let webhook = matches
            .opt_str("w")
            .unwrap_or(String::from("0"))
            .parse::<u16>()
            .unwrap_or(0);

        Ok(Self {
            token,
            proxy,
            webhook,
            file,
        })
    }
}
