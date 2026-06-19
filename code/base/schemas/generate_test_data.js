#!/usr/bin/env node
/**
 * Generate Test Data for Products Schema
 *
 * Generates realistic test data matching the UI expectations:
 * - ProductCard interface (for listings)
 * - Product interface (for details)
 *
 * Output: SQL file ready to import
 */

const fs = require('fs');
const path = require('path');

// Configuration
const NUM_PRODUCTS = 2000;
const NUM_RATINGS_PER_PRODUCT = { min: 5, max: 50 };

// Sample data pools
const BRANDS = [
  'TechPro', 'SmartGear', 'EliteHome', 'ActiveLife', 'StyleHub',
  'PureTech', 'FlexFit', 'MegaValue', 'ProMax', 'UltraGear',
  'CoreTech', 'PrimeTech', 'NexGen', 'FutureTech', 'DigitalWave'
];

const CATEGORIES = [
  { name: 'Smartphones', root: 'Electronics', adjectives: ['Pro', 'Max', 'Ultra', 'Plus', 'Elite'] },
  { name: 'Laptops', root: 'Electronics', adjectives: ['Book', 'Pro', 'Air', 'Gaming', 'Business'] },
  { name: 'Accessories', root: 'Electronics', adjectives: ['Premium', 'Wireless', 'Smart', 'Pro', 'Plus'] },
  { name: 'Smart Home', root: 'Electronics', adjectives: ['Smart', 'Connected', 'Voice', 'AI', 'Auto'] },
  { name: 'Men\'s Clothing', root: 'Clothing', adjectives: ['Classic', 'Modern', 'Casual', 'Sport', 'Premium'] },
  { name: 'Women\'s Clothing', root: 'Clothing', adjectives: ['Elegant', 'Casual', 'Sport', 'Designer', 'Comfort'] },
  { name: 'Shoes', root: 'Clothing', adjectives: ['Running', 'Walking', 'Sport', 'Casual', 'Athletic'] }
];

const PRODUCT_TYPES = {
  'Smartphones': ['Phone', 'Smartphone', 'Mobile Device', 'Handset'],
  'Laptops': ['Laptop', 'Notebook', 'Ultrabook', 'Chromebook'],
  'Accessories': ['Cable', 'Charger', 'Case', 'Stand', 'Mount', 'Adapter', 'Hub'],
  'Smart Home': ['Camera', 'Doorbell', 'Thermostat', 'Lock', 'Speaker', 'Light'],
  'Men\'s Clothing': ['Shirt', 'T-Shirt', 'Pants', 'Jeans', 'Jacket', 'Sweater'],
  'Women\'s Clothing': ['Dress', 'Blouse', 'Skirt', 'Pants', 'Jacket', 'Sweater'],
  'Shoes': ['Sneakers', 'Boots', 'Sandals', 'Loafers', 'Athletic Shoes']
};

const SIZES = {
  clothing: ['XS', 'S', 'M', 'L', 'XL', 'XXL'],
  shoes: ['6', '7', '8', '9', '10', '11', '12'],
  tech: ['64GB', '128GB', '256GB', '512GB', '1TB']
};

const COLORS = ['Black', 'White', 'Silver', 'Gold', 'Blue', 'Red', 'Green', 'Gray', 'Rose Gold', 'Navy'];

const DESCRIPTIONS = [
  'High-quality product designed for everyday use.',
  'Premium build quality with advanced features.',
  'Perfect for both professional and personal use.',
  'Innovative design combining style and functionality.',
  'Top-rated product with excellent performance.',
  'Durable construction built to last.',
  'Feature-rich product at an affordable price.',
  'Award-winning design with cutting-edge technology.',
  'User-friendly interface with powerful capabilities.',
  'Sleek and modern design for the contemporary lifestyle.'
];

// Helper functions
function randomInt(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function randomChoice(array) {
  return array[randomInt(0, array.length - 1)];
}

function randomChoices(array, count) {
  const shuffled = [...array].sort(() => 0.5 - Math.random());
  return shuffled.slice(0, count);
}

function generateUUID() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

function generateASIN() {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  return Array.from({ length: 10 }, () => randomChoice([...chars])).join('');
}

function sqlString(str) {
  if (str === null || str === undefined) return 'NULL';
  return `'${str.replace(/'/g, "''")}'`;
}

function sqlArray(arr) {
  if (!arr || arr.length === 0) return 'NULL';
  return `ARRAY[${arr.map(sqlString).join(', ')}]`;
}

function calculateDiscount(initial, final) {
  if (!initial || initial === final) return null;
  const percent = Math.round(((initial - final) / initial) * 100);
  return `${percent}%`;
}

// Generate products
function generateProducts() {
  const products = [];
  const productsByCategory = new Map();

  for (let i = 0; i < NUM_PRODUCTS; i++) {
    const category = randomChoice(CATEGORIES);
    const brand = randomChoice(BRANDS);
    const productType = randomChoice(PRODUCT_TYPES[category.name] || ['Product']);
    const adjective = randomChoice(category.adjectives);
    const modelNumber = randomInt(1000, 9999);

    // Generate pricing
    const basePrice = randomInt(10, 2000);
    const hasDiscount = Math.random() > 0.6; // 40% of products have discount
    const finalPrice = hasDiscount ? basePrice * (1 - randomInt(10, 40) / 100) : basePrice;
    const initialPrice = hasDiscount ? basePrice : null;

    // Generate sizes and colors based on category
    let sizes = null;
    let colors = null;

    if (category.root === 'Clothing' && category.name !== 'Shoes') {
      sizes = randomChoices(SIZES.clothing, randomInt(3, 6));
      colors = randomChoices(COLORS, randomInt(2, 4));
    } else if (category.name === 'Shoes') {
      sizes = randomChoices(SIZES.shoes, randomInt(5, 7));
      colors = randomChoices(COLORS, randomInt(1, 3));
    } else if (category.name === 'Smartphones' || category.name === 'Laptops') {
      sizes = randomChoices(SIZES.tech, randomInt(2, 4));
      colors = randomChoices(COLORS.slice(0, 5), randomInt(2, 4));
    } else {
      colors = randomChoices(COLORS.slice(0, 5), randomInt(1, 3));
    }

    const product = {
      product_id: `PROD-${String(i + 1).padStart(6, '0')}`,
      uuid: generateUUID(),
      asin: generateASIN(),
      sku: `${category.name.substring(0, 3).toUpperCase()}-${brand.substring(0, 3).toUpperCase()}-${String(i + 1).padStart(6, '0')}`,
      gtin: Math.random() > 0.5 ? `${randomInt(1000000000000, 9999999999999)}` : null,
      product_name: `${brand} ${adjective} ${productType} ${modelNumber}`,
      brand: brand,
      description: randomChoice(DESCRIPTIONS),
      category_name: category.name,
      root_category_name: category.root,
      price: parseFloat(finalPrice.toFixed(2)),
      initial_price: initialPrice ? parseFloat(initialPrice.toFixed(2)) : null,
      discount: calculateDiscount(initialPrice, finalPrice),
      currency: 'USD',
      stock_quantity: randomInt(0, 500),
      sizes: sizes,
      colors: colors,
      url: `https://example.com/product/${i + 1}`,
      image_url: null,
      available_for_delivery: Math.random() > 0.1,
      available_for_pickup: Math.random() > 0.5,
      free_returns: Math.random() > 0.4,
      is_active: Math.random() > 0.05,
      deleted_at: null,
      data_timestamp: new Date().toISOString()
    };

    products.push(product);

    if (!productsByCategory.has(category.name)) {
      productsByCategory.set(category.name, []);
    }
    productsByCategory.get(category.name).push(product);
  }

  return { products, productsByCategory };
}

// Generate ratings
function generateRatings(products) {
  const ratings = [];
  let ratingId = 1;

  products.forEach((product, idx) => {
    const numRatings = randomInt(NUM_RATINGS_PER_PRODUCT.min, NUM_RATINGS_PER_PRODUCT.max);

    for (let i = 0; i < numRatings; i++) {
      const rating = {
        uuid: generateUUID(),
        product_id: idx + 1, // Will be the serial ID
        user_id: generateUUID(),
        rating: randomInt(1, 5),
        review: Math.random() > 0.6 ? randomChoice([
          'Great product! Highly recommended.',
          'Good value for money.',
          'Exceeded my expectations.',
          'Exactly as described.',
          'Fast shipping and great quality.',
          'Not bad, but could be better.',
          'Satisfied with the purchase.',
          'Works perfectly!',
          'Very happy with this product.',
          'Would buy again.'
        ]) : null
      };

      ratings.push(rating);
      ratingId++;
    }
  });

  return ratings;
}

// Generate SQL
function generateSQL(products, ratings) {
  let sql = `-- Generated test data for products schema
-- Total products: ${products.length}
-- Total ratings: ${ratings.length}
-- Generated at: ${new Date().toISOString()}

SET search_path TO products, public;

-- ============================================================
-- INSERT PRODUCTS
-- ============================================================
INSERT INTO products (
  product_id, uuid, asin, sku, gtin, product_name, brand, description,
  category_id, price, initial_price, discount, currency, stock_quantity,
  sizes, colors, url, image_url,
  available_for_delivery, available_for_pickup, free_returns,
  is_active, deleted_at, data_timestamp
) VALUES\n`;

  // Insert products
  products.forEach((p, idx) => {
    sql += `  (`;
    sql += `${sqlString(p.product_id)}, `;
    sql += `${sqlString(p.uuid)}, `;
    sql += `${sqlString(p.asin)}, `;
    sql += `${sqlString(p.sku)}, `;
    sql += `${sqlString(p.gtin)}, `;
    sql += `${sqlString(p.product_name)}, `;
    sql += `${sqlString(p.brand)}, `;
    sql += `${sqlString(p.description)}, `;
    sql += `(SELECT id FROM categories WHERE name = ${sqlString(p.category_name)}), `;
    sql += `${p.price}, `;
    sql += `${p.initial_price || 'NULL'}, `;
    sql += `${sqlString(p.discount)}, `;
    sql += `${sqlString(p.currency)}, `;
    sql += `${p.stock_quantity}, `;
    sql += `${sqlArray(p.sizes)}, `;
    sql += `${sqlArray(p.colors)}, `;
    sql += `${sqlString(p.url)}, `;
    sql += `${sqlString(p.image_url)}, `;
    sql += `${p.available_for_delivery}, `;
    sql += `${p.available_for_pickup}, `;
    sql += `${p.free_returns}, `;
    sql += `${p.is_active}, `;
    sql += `${sqlString(p.deleted_at)}, `;
    sql += `${sqlString(p.data_timestamp)}`;
    sql += `)${idx < products.length - 1 ? ',' : ';'}\n`;
  });

  sql += `\n-- ============================================================\n`;
  sql += `-- INSERT RATINGS\n`;
  sql += `-- ============================================================\n`;
  sql += `INSERT INTO ratings (uuid, product_id, user_id, rating, review) VALUES\n`;

  // Insert ratings
  ratings.forEach((r, idx) => {
    sql += `  (`;
    sql += `${sqlString(r.uuid)}, `;
    sql += `${r.product_id}, `;
    sql += `${sqlString(r.user_id)}, `;
    sql += `${r.rating}, `;
    sql += `${sqlString(r.review)}`;
    sql += `)${idx < ratings.length - 1 ? ',' : ';'}\n`;
  });

  return sql;
}

// Main execution
console.log('Generating test data...');
console.log(`- Products: ${NUM_PRODUCTS}`);
console.log(`- Ratings per product: ${NUM_RATINGS_PER_PRODUCT.min}-${NUM_RATINGS_PER_PRODUCT.max}`);

const { products, productsByCategory } = generateProducts();
const ratings = generateRatings(products);

console.log('\nGenerated:');
console.log(`  ${products.length} products`);
console.log(`  ${ratings.length} ratings`);
console.log('\nProducts by category:');
productsByCategory.forEach((prods, cat) => {
  console.log(`  ${cat}: ${prods.length}`);
});

const sql = generateSQL(products, ratings);
const outputPath = path.join(__dirname, 'data', 'products_test_data.sql');

fs.writeFileSync(outputPath, sql);
console.log(`\nSQL file generated: ${outputPath}`);
console.log('\nTo load data:');
console.log(`  psql -h localhost -p 5433 -U opentel_user -d opentel_db -f ${outputPath}`);
