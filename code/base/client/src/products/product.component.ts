import { CommonModule } from '@angular/common';
import { Component, computed, ElementRef, inject, input, signal, viewChild } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { CartService } from '../services/cart.service';

const morphAndFadeKeyframes = (sx: string, sy: string) => {
  const rect = window.cartBtnPosition();
  const dx = `${rect.x}px`;
  const dy = `${rect.y}px`;
  console.log({ sx, sy, dx, dy });
  return [
    // 0%
    {
      opacity: 0,
      transform: `translate(${sx}, ${sy}) scale(.75)`,
      offset: 0.0,
    },
    // 8%
    {
      opacity: 0.5,
      transform: `translate(${sx}, ${sy}) scale(1)`,
      offset: 0.08,
    },
    // 20%
    {
      opacity: 1,
      transform: `translate(${sx}, ${sy}) scale(1.5)`,
      offset: 0.2,
    },
    // 80% (Movement to destination, maintaining scale and opacity)
    {
      opacity: 1,
      transform: `translate(${dx}, ${dy}) scale(1.25)`,
      offset: 0.8,
    },
    // 90%
    {
      opacity: 0.75,
      transform: `translate(${dx}, ${dy}) scale(1)`,
      offset: 0.8,
    },
    {
      opacity: 0.5,
      transform: `translate(${dx}, ${dy}) scale(.5)`,
      offset: 0.9,
    },
    // 100%
    {
      opacity: 0,
      transform: `translate(${dx}, ${dy}) scale(.1)`,
      offset: 1.0,
    },
  ];
};

@Component({
  selector: 'div.product',
  templateUrl: './product.component.html',
  styleUrls: ['./product.component.scss'],
  standalone: true,
  imports: [CommonModule, RouterLink],
})
export class ProductComponent {
  private cartService = inject(CartService);
  readonly product = input<ProductCard>({} as ProductCard);
  readonly addingToCart = signal<boolean>(false);
  private morphDiv = viewChild<ElementRef<HTMLDivElement>>('morphDiv');
  private div = computed(() => this.morphDiv()?.nativeElement as HTMLDivElement);

  ratingStars = computed(() => {
    const rating = this.product().average_rating || 0;
    return rating === 0 ? '' : Number.isInteger(rating) ? '-fill' : '-half';
  });

  get discount(): number {
    const initial = parseFloat(this.product().initial_price || '0');
    const discount = parseFloat(this.product().discount || '0');
    return initial > 0 ? Math.round((discount / initial) * 100) : 0;
  }

  addToCart($event: Event): void {
    $event.preventDefault();
    $event.stopPropagation();
    const btn = $event.target as HTMLButtonElement;

    this.addingToCart.set(true);
    const options = {
      duration: 1000, // Example duration in milliseconds (adjust as needed)
      iterations: 1,
      fill: 'forwards', // Keeps the final state (opacity: 0, scaled down, at destination)
      easing: 'cubic-bezier(0.42, 0, 0.58, 1)', // Applies a general easing across the duration
    } as KeyframeAnimationOptions;

    const { x, y, width, height } = btn.getBoundingClientRect();
    const keyframes = morphAndFadeKeyframes(`${x + 10}px`, `${y - 3}px`);
    const div = document.body.appendChild(this.div().cloneNode(true) as HTMLDivElement);
    const animation = div.animate(keyframes, options);
    animation.onfinish = () => {
      this.cartService.addToCart(this.product());
      this.addingToCart.set(false);
      div.remove();
    };
  }
}
