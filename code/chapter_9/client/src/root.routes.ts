import { inject } from '@angular/core';
import { Route, Routes } from '@angular/router';
import { CartService } from './services/cart.service';
import { NotificationService } from './services/notification.service';

export const routes: Routes = [
  {
    path: '',
    redirectTo: 'products',
    pathMatch: 'full',
  },
  {
    path: 'products',
    loadChildren: () => import('./products/products.routes').then((m) => m.PRODUCT_ROUTES),
  },
  {
    path: 'checkout',
    loadComponent: () => import('./cart/checkout.component').then((m) => m.CheckoutComponent),
    canActivate: [
      () => {
        const isEmpty = inject(CartService).isEmpty();
        if (isEmpty) {
          inject(NotificationService).show(
            'Your cart is empty. Please add items before proceeding to checkout.',
            'danger',
          );
        }
        return !isEmpty;
      },
    ],
  },
  {
    path: '**',
    redirectTo: 'products',
  },
];
