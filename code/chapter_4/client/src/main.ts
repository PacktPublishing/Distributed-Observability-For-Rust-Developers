import { bootstrapApplication } from '@angular/platform-browser';
import { appConfig } from './root.config';
import { App } from './app';

bootstrapApplication(App, appConfig)
  .catch((err) => console.error(err));
