#!/usr/bin/env node

/**
 * Generate realistic product test data for OtelMart
 *
 * This script generates product data that matches the UI expectations:
 * - ProductCard interface (for list view)
 * - Product interface (for detail view)
 *
 * Generated fields:
 * - Basic info: name, brand, description
 * - Pricing: price, initial_price, discount
 * - Inventory: stock_quantity
 * - Categories: with hierarchical support
 * - Attributes: sizes, colors (for applicable products)
 * - Ratings: sample ratings for some products
 */

const fs = require('fs');
const {randomUUID} = require('crypto');

// =====================================================================
// Configuration
// =====================================================================

const NUM_PRODUCTS = 2000;
const NUM_RATINGS = 500; // Number of sample ratings to generate
const OUTPUT_FILE = './test_data.sql';

// =====================================================================
// Data Templates
// =====================================================================

const BRANDS = [
  'TechPro', 'CoreTech', 'SmartLife', 'HomeEssentials', 'StyleHub',
  'SportMax', 'EcoGear', 'PremiumChoice', 'ValueBrand', 'EliteGoods',
  'FastTrack', 'PureQuality', 'ModernHome', 'ActiveLife', 'ComfortZone'
];

const PRODUCT_TYPES = {
  smartphones: ['iPhone Pro', 'Galaxy Ultra', 'Pixel Premium', 'OnePlus Flagship'],
  laptops: ['MacBook Pro', 'ThinkPad Elite', 'XPS Premium', 'Surface Laptop'],
  accessories: ['Wireless Mouse', 'Keyboard', 'USB Hub', 'Phone Case', 'Screen Protector', 'Charger', 'Cable', 'Earbuds'],
  smartHome: ['Smart Speaker', 'Security Camera', 'Thermostat', 'Video Doorbell', 'Light Bulb', 'Smart Lock'],
  mensClothing: ['T-Shirt', 'Jeans', 'Jacket', 'Hoodie', 'Shirt', 'Pants'],
  womensClothing: ['Dress', 'Blouse', 'Skirt', 'Leggings', 'Sweater'],
  shoes: ['Running Shoes', 'Sneakers', 'Boots', 'Sandals'],
  homeGarden: ['Garden Tools', 'Furniture', 'Decor', 'Kitchen Set'],
  sports: ['Yoga Mat', 'Dumbbells', 'Resistance Bands', 'Treadmill'],
  books: ['Fiction Novel', 'Cookbook', 'Self-Help Book', 'Technical Manual'],
  health: ['Vitamins', 'Face Cream', 'Shampoo', 'Supplements'],
  toys: ['Board Game', 'Action Figure', 'Puzzle', 'LEGO Set'],
  automotive: ['Car Cover', 'Floor Mats', 'Phone Mount', 'HDMI Cable']
};

const ADJECTIVES = [
  'Premium', 'Elite', 'Pro', 'Ultra', 'Smart', 'Eco-Friendly',
  'Wireless', 'Portable', 'Compact', 'Heavy-Duty', 'Deluxe',
  'Advanced', 'Professional', 'Classic', 'Modern', 'Innovative'
];

const SIZES = {
  clothing: ['XS', 'S', 'M', 'L', 'XL', 'XXL'],
  shoes: ['7', '8', '9', '10', '11', '12'],
  default: []
};

const COLORS = {
  electronics: ['Black', 'Silver', 'White', 'Space Gray'],
  clothing: ['Black', 'White', 'Navy', 'Gray', 'Red', 'Blue', 'Green'],
  shoes: ['Black', 'White', 'Brown', 'Gray'],
  default: []
};

// Map product types to categories (will be replaced with DB category IDs)
const CATEGORY_MAP = {
  smartphones: 'Smartphones',
  laptops: 'Laptops',
  accessories: 'Accessories',
  smartHome: 'Smart Home',
  mensClothing: "Men's Clothing",
  womensClothing: "Women's Clothing",
  shoes: 'Shoes',
  homeGarden: 'Home & Garden',
  sports: 'Sports & Outdoors',
  books: 'Books & Media',
  health: 'Health & Beauty',
  toys: 'Toys & Games',
  automotive: 'Automotive'
};

// =====================================================================
// Helper Functions
// =====================================================================

function randomInt(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function randomElement(array) {
  return array[randomInt(0, array.length - 1)];
}

function randomBool(probability = 0.5) {
  return Math.random() < probability;
}

let skuCounter = 10000;
function generateSKU() {
  const prefix = randomElement(['PRD', 'SKU', 'ITM']);
  return `${prefix}-${skuCounter++}`;
}

let asinCounter = 100000;
function generateASIN() {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  let asin = 'B';
  const counterStr = asinCounter.toString().padStart(8, '0');
  asin += counterStr;
  const checkChar = chars[asinCounter % chars.length];
  asin += checkChar;
  asinCounter++;
  return asin;
}

function generateDescription(productName, brand) {
  const templates = [
    `${productName} from ${brand}. High-quality product with excellent features and durability.`,
    `Experience the best with this ${productName}. Perfect for everyday use and built to last.`,
    `${brand}'s ${productName} combines style and functionality in one amazing package.`,
    `Premium ${productName} designed for maximum performance and user satisfaction.`,
    `Innovative ${productName} featuring cutting-edge technology and modern design.`
  ];
  return randomElement(templates);
}

function generatePrice() {
  const basePrice = randomInt(10, 500);
  const cents = randomElement([99, 95, 50, 00]);
  return `${basePrice}.${cents.toString().padStart(2, '0')}`;
}

function calculateDiscount(initialPrice, finalPrice) {
  const initial = parseFloat(initialPrice);
  const final = parseFloat(finalPrice);

  if (initial <= final) return null;

  const percentage = ((initial - final) / initial * 100).toFixed(0);
  return `${percentage}%`;
}

function getSizesForCategory(category) {
  if (category.includes('Clothing')) return SIZES.clothing;
  if (category.includes('Shoes')) return SIZES.shoes;
  return SIZES.default;
}

function getColorsForCategory(category) {
  if (category.includes('Electronics') || category.includes('Smart')) return COLORS.electronics;
  if (category.includes('Clothing')) return COLORS.clothing;
  if (category.includes('Shoes')) return COLORS.shoes;
  return COLORS.default;
}

// =====================================================================
// Product Generation
// =====================================================================

function generateProduct(id, typeKey, category) {
  const productTypes = PRODUCT_TYPES[typeKey];
  const baseName = randomElement(productTypes);
  const adjective = randomBool(0.6) ? randomElement(ADJECTIVES) + ' ' : '';
  const number = randomInt(1000, 9999);
  const brand = randomElement(BRANDS);

  const productName = `${adjective}${baseName} ${number}`;
  const description = generateDescription(baseName, brand);

  // Pricing logic: 30% of products have discounts
  const hasDiscount = randomBool(0.3);
  let finalPrice, initialPrice, discount;

  if (hasDiscount) {
    initialPrice = generatePrice();
    const discountPercent = randomInt(10, 50);
    const final = parseFloat(initialPrice) * (1 - discountPercent / 100);
    finalPrice = final.toFixed(2);
    discount = calculateDiscount(initialPrice, finalPrice);
  } else {
    finalPrice = generatePrice();
    initialPrice = null;
    discount = null;
  }

  // Stock
  const stockQuantity = randomInt(0, 500);

  // Sizes and colors
  const sizes = getSizesForCategory(category);
  const colors = getColorsForCategory(category);

  const sizesArray = sizes.length > 0 && randomBool(0.7) ?
    `'{${sizes.slice(0, randomInt(2, sizes.length)).join(',')}}'` :
    'NULL';

  const colorsArray = colors.length > 0 && randomBool(0.8) ?
    `'{${colors.slice(0, randomInt(2, colors.length)).join(',')}}'` :
    'NULL';

  // Flags
  const availableForDelivery = randomBool(0.95);
  const availableForPickup = randomBool(0.3);
  const freeReturns = randomBool(0.6);
  const isActive = randomBool(0.98);

  return {
    id,
    uuid: randomUUID(),
    asin: generateASIN(),
    sku: generateSKU(),
    productName,
    brand,
    description,
    category,
    price: finalPrice,
    initialPrice,
    discount,
    stockQuantity,
    sizes: sizesArray,
    colors: colorsArray,
    availableForDelivery,
    availableForPickup,
    freeReturns,
    isActive
  };
}

// =====================================================================
// Generate SQL
// =====================================================================

function escapeSQL(str) {
  if (str === null || str === undefined) return 'NULL';
  return `'${str.replace(/'/g, "''")}'`;
}

function generateProductSQL(product) {
  return `(
    ${escapeSQL(product.uuid)},
    ${escapeSQL(product.asin)},
    ${escapeSQL(product.sku)},
    ${escapeSQL(product.productName)},
    ${escapeSQL(product.brand)},
    ${escapeSQL(product.description)},
    (SELECT id FROM categories WHERE name = ${escapeSQL(product.category)}),
    ${product.price},
    ${product.initialPrice ? product.initialPrice : 'NULL'},
    ${escapeSQL(product.discount)},
    'USD',
    ${product.stockQuantity},
    ${product.sizes},
    ${product.colors},
    ${product.availableForDelivery},
    ${product.availableForPickup},
    ${product.freeReturns},
    ${product.isActive}
  )`;
}

function generateRatingSQL(productId, userId, rating, review) {
  return `(
    ${escapeSQL(randomUUID())},
    ${productId},
    ${escapeSQL(userId)},
    ${rating},
    ${escapeSQL(review)}
  )`;
}

// =====================================================================
// Main Generation
// =====================================================================

console.log('Generating test product data...');

const products = [];
let productId = 1;

// Distribute products across categories
const typeKeys = Object.keys(PRODUCT_TYPES);
const productsPerType = Math.floor(NUM_PRODUCTS / typeKeys.length);

typeKeys.forEach((typeKey, index) => {
  const count = index === typeKeys.length - 1 ?
    NUM_PRODUCTS - products.length : // Last type gets remainder
    productsPerType;

  const category = CATEGORY_MAP[typeKey];

  for (let i = 0; i < count; i++) {
    products.push(generateProduct(productId++, typeKey, category));
  }
});

console.log(`Generated ${products.length} products`);

// Generate SQL file
let sql = `-- =====================================================================
-- OTELMART TEST DATA
-- Generated: ${new Date().toISOString()}
-- Products: ${products.length}
-- Ratings: ${NUM_RATINGS}
-- =====================================================================

SET search_path TO products, public;

-- =====================================================================
-- INSERT PRODUCTS
-- =====================================================================

INSERT INTO products (
  uuid, asin, sku, product_name, brand, description,
  category_id, price, initial_price, discount, currency,
  stock_quantity, sizes, colors,
  available_for_delivery, available_for_pickup, free_returns, is_active
) VALUES\n`;

sql += products.map((p, i) =>
  generateProductSQL(p) + (i < products.length - 1 ? ',' : ';')
).join('\n');

// Generate sample ratings
sql += `\n\n-- =====================================================================
-- INSERT SAMPLE RATINGS
-- =====================================================================

INSERT INTO ratings (uuid, product_id, user_id, rating, review) VALUES\n`;

const ratingReviews = [
  'Great product! Highly recommended.',
  'Excellent quality and fast shipping.',
  'Good value for the price.',
  'Works perfectly as described.',
  'Very satisfied with this purchase.',
  'Amazing! Better than expected.',
  'Decent product, does the job.',
  'Not bad, but could be better.',
  'Fantastic quality!',
  null // Some ratings without reviews
];

const ratings = [];
for (let i = 0; i < NUM_RATINGS; i++) {
  const productId = randomInt(1, products.length);
  const userId = randomUUID();
  const rating = randomInt(3, 5); // Mostly positive ratings
  const review = randomElement(ratingReviews);

  ratings.push(generateRatingSQL(productId, userId, rating, review));
}

sql += ratings.map((r, i) =>
  r + (i < ratings.length - 1 ? ',' : ';')
).join('\n');

sql += '\n\n-- Data generation complete\n';

// Write to file
fs.writeFileSync(OUTPUT_FILE, sql);

console.log(`\nTest data generated successfully!`);
console.log(`Output file: ${OUTPUT_FILE}`);
console.log(`\nTo import:`);
console.log(`psql -h localhost -p 5433 -U opentel_user -d opentel_db -f ${OUTPUT_FILE}`);
