import { CommonModule } from '@angular/common';
import { Component, input, output, signal, effect } from '@angular/core';
import { ReactiveFormsModule, FormGroup, FormControl, Validators } from '@angular/forms';

@Component({
  selector: 'section.search',
  templateUrl: './search.component.html',
  styleUrls: ['./search.component.scss'],
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
})
export class SearchComponent {
  // Input for controlling search state from parent (when parent wants to reset)
  readonly searching = input<boolean>(false);

  // Internal searching state
  readonly isSearching = signal<boolean>(false);

  // Event emitter for search value changes (emitted on form submit)
  searchValueChange = output<string>();

  // Reactive form group
  searchForm = new FormGroup({
    search: new FormControl('', [Validators.minLength(2)]),
  });

  constructor() {
    // Effect to watch for reset signal from parent
    effect(() => {
      if (this.searching()) {
        this.isSearching.set(false);
      }
    });
  }

  onSearchSubmit(): void {
    const searchTerm = this.searchForm.get('search')?.value?.trim();
    if (this.searchForm.invalid || !searchTerm) {
      return;
    }

    // Set searching state to true immediately
    this.isSearching.set(true);
    // Emit the search value and let parent handle the search logic
    this.searchValueChange.emit(searchTerm);
  }
}
