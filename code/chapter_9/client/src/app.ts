import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { HeaderComponent } from './header/header.component';
import { CartComponent } from './cart/cart.component';
import { NotificationComponent } from './services/notification.component';

@Component({
  selector: 'body',
  templateUrl: './app.html',
  styleUrls: ['./app.scss'],
  imports: [RouterOutlet, HeaderComponent, CartComponent, NotificationComponent],
})
export class App {}
