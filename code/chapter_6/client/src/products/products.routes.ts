import { Routes } from '@angular/router';
import { ProductsListComponent } from './products-list.component';
import { ProductDetailsComponent } from './product-details.component';

export const PRODUCT_ROUTES: Routes = [
    {
        path: '',
        component: ProductsListComponent,        
    },
    {
        path: ':productId',
        component: ProductDetailsComponent,
    }
];