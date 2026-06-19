import { Component, input, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, ActivatedRoute } from '@angular/router';

@Component({
  selector: 'div.pagination',
  templateUrl: './paginate.component.html',
  styleUrls: ['./paginate.component.scss'],
  standalone: true,
  imports: [CommonModule],
})
export class PaginateComponent {
  private router = inject(Router);
  private route = inject(ActivatedRoute);

  // Inputs from parent
  readonly totalPages = input.required<number>();
  readonly total = input.required<number>();
  readonly page = input.required<number>();
  readonly pageSize = input.required<number>();

  // Available page sizes
  readonly pageSizes = [10, 20, 50, 100];

  // Computed properties
  readonly canGoPrevious = computed(() => this.page() > 1);
  readonly canGoNext = computed(() => this.page() < this.totalPages());
  readonly startItem = computed(() => (this.page() - 1) * this.pageSize() + 1);
  readonly endItem = computed(() => Math.min(this.page() * this.pageSize(), this.total()));

  // Update URL query params - Angular inputs will auto-update from URL changes
  private navigate(page: number, pageSize = this.pageSize()): void {
    this.router.navigate([], {
      relativeTo: this.route,
      queryParams: { page, pageSize },
      queryParamsHandling: 'merge',
    });
  }

  // Get visible page numbers with smart ellipsis
  getVisiblePages(): (number | string)[] {
    const total = this.totalPages();
    const current = this.page();

    if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);

    const pages: (number | string)[] = [1];

    if (current <= 4) {
      pages.push(...[2, 3, 4, 5], '...', total);
    } else if (current >= total - 3) {
      pages.push('...', ...Array.from({ length: 5 }, (_, i) => total - 4 + i));
    } else {
      pages.push('...', current - 1, current, current + 1, '...', total);
    }

    return pages;
  }

  // Navigation methods
  goToPage(page: number): void {
    if (page >= 1 && page <= this.totalPages()) {
      this.navigate(page);
    }
  }

  onPageSizeChange(event: Event): void {
    const size = +(event.target as HTMLSelectElement).value;
    if (this.pageSizes.includes(size)) {
      this.navigate(1, size); // Reset to page 1 when changing page size
    }
  }
}
