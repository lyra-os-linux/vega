import { N_ } from "../i18n.js";
import dashboard from "./dashboard.js";
import software from "./software.js";
import backup from "./backup.js";
import virtualization from "./virtualization.js";
import assistant from "./assistant.js";
import hardware from "./hardware.js";
import datetime from "./datetime.js";
import personalization from "./personalization.js";
import monitor from "./monitor.js";
import storage from "./storage.js";
import network from "./network.js";
import bluetooth from "./bluetooth.js";
import services from "./services.js";
import users from "./users.js";
import logs from "./logs.js";

// Mesmos ids de página do vega-gtk: `start_page` das preferências continua valendo.
export const groups = [
  { id: "main", title: N_("Principal") },
  { id: "system", title: N_("Sistema") },
];

export const pages = [dashboard, software, backup, virtualization, assistant, hardware, datetime, personalization, monitor, storage, network, bluetooth, services, users, logs];
