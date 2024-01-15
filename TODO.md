# TODO

## Now

- Look for TODO comments in the code and complete the task
- Auto increment version
- If car name is changed, the change doesn't get updated in the database. Periodically check car's configuration and update the database if there are any changes.

## Later

- Switch from nightly to stable rust when 'async_closure' and 'let_chains' features are stable
- Remove tesla key from .env
    - Add a textbox to web interface to enter the key
    - Remove the key from .env
- Add option to select log level
    - Add option to web interface
    - Use docker environment to select log level
- Update readme
    - Add introduction
    - Mention teslamate and teslalogger projects
- Update charging status
    - Pull supercharging info from Tesla's web page and add charging details to database
    - Integrate with chargepoint
    - Make it easier to add more charging providers in future
- Create better logo and icon
- Replace openssl crate with something simpler
- Fix the panic when the http oprt 3072 is being used by another process
- Update car table in database every day
- If vehicle is offline, wait for it to wake up instead of polling for data
