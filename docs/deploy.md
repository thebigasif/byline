# Running Khalid on a server

Khalid is one file. Put it on a machine, point it at a site folder, and it serves the blog and the admin. This page shows the ways to keep it running. The files it names are in `packaging/` in the repository.

A site folder is the folder that holds `site.toml`, `content/` and `theme/`. Khalid keeps what it works out from those files, such as its database and image sizes, in `data/` inside the same folder. Back up the site folder and you have backed up the blog.

## systemd on a Linux machine

systemd is the program most Linux systems use to start services at boot. Khalid ships a unit file for it, `packaging/systemd/khalid.service`, and a user file, `packaging/systemd/khalid.sysusers`.

The unit runs Khalid as its own `khalid` user, never as root. The site folder is `/var/lib/khalid`, and the service can write nowhere else. It answers on `127.0.0.1:8080`, so only a program on the same machine, such as a proxy, can reach it.

1. Put the binary at `/usr/local/bin/khalid`.
2. Copy the two files and make the user:

   ```sh
   sudo cp packaging/systemd/khalid.service /etc/systemd/system/
   sudo cp packaging/systemd/khalid.sysusers /etc/sysusers.d/khalid.conf
   sudo systemd-sysusers
   ```

3. Make the site, as the `khalid` user:

   ```sh
   sudo install -d -o khalid -g khalid -m 0750 /var/lib/khalid
   sudo -u khalid khalid init /var/lib/khalid --address https://blog.example.com --yes
   ```

4. Start it now and at every boot:

   ```sh
   sudo systemctl enable --now khalid
   ```

`systemctl status khalid` says whether it runs, and `journalctl -u khalid` shows what it printed.

To change the command, for example to answer on every address of the machine, run `sudo systemctl edit khalid` and write:

```ini
[Service]
ExecStart=
ExecStart=/usr/local/bin/khalid --site /var/lib/khalid serve --listen 0.0.0.0:8080
```

The empty `ExecStart=` line clears the old command first. The unit already lets Khalid answer on ports 80 and 443 without root, for when it gets its own certificate with `serve --domain`.
