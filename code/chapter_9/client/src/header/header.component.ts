import { Component, ElementRef, viewChild, AfterViewInit, inject } from '@angular/core';
import { UserProfileComponent } from './user-profile.component';
import { SearchComponent } from './search.component';
import { ClickOutsideDirective } from '../components/click-outside.directive';
import { CartService } from '../services/cart.service';

@Component({
  selector: 'header.app-header',
  templateUrl: './header.component.html',
  styleUrls: ['./header.component.scss'],
  standalone: true,
  imports: [UserProfileComponent, SearchComponent, ClickOutsideDirective],
})
export class HeaderComponent implements AfterViewInit {
  readonly cartService = inject(CartService);
  
  cartBtn = viewChild<ElementRef<HTMLButtonElement>>('cartBtn');

  ngAfterViewInit() {
    const cartButton = this.cartBtn()?.nativeElement;
    if (cartButton) {
      let cachedRect: DOMRect | null = null;
      window.cartBtnPosition = () => {
        if (!cachedRect) {
          cachedRect = cartButton.getBoundingClientRect();
        }
        return cachedRect;
      };
    }
  }
}
