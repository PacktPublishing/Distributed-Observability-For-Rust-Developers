#!/usr/bin/env node

/**
 * Generate 2000 product records for the products service
 * Outputs SQL INSERT statements to schemas/data/products_data.sql
 */

const fs = require('fs');
const path = require('path');

// Product categories
const categories = [
    'Electronics', 'Accessories', 'Gaming', 'Office',
    'Audio', 'Computers', 'Mobile', 'Wearables',
    'Smart Home', 'Camera', 'Networking', 'Storage'
];

// Product name templates
const productTypes = {
    Electronics: [
        'Monitor', 'Laptop', 'Desktop', 'Tablet', 'E-Reader',
        'TV', 'Projector', 'Smart Display', 'Graphics Card', 'Motherboard'
    ],
    Accessories: [
        'USB-C Cable', 'HDMI Cable', 'Power Adapter', 'Phone Case',
        'Screen Protector', 'Cable Organizer', 'Laptop Sleeve', 'USB Hub'
    ],
    Gaming: [
        'Gaming Mouse', 'Gaming Keyboard', 'Gaming Headset', 'Controller',
        'Gaming Chair', 'Mouse Pad', 'VR Headset', 'Gaming Monitor', 'Capture Card'
    ],
    Office: [
        'Desk Lamp', 'Office Chair', 'Standing Desk', 'Whiteboard',
        'Desk Organizer', 'Notebook', 'Pen Set', 'Stapler', 'Paper Shredder'
    ],
    Audio: [
        'Bluetooth Speaker', 'Headphones', 'Earbuds', 'Microphone',
        'Sound Bar', 'Subwoofer', 'Audio Interface', 'Studio Monitor'
    ],
    Computers: [
        'Keyboard', 'Mouse', 'Webcam', 'External Drive', 'RAM Module',
        'SSD', 'CPU', 'Cooling Fan', 'Power Supply', 'PC Case'
    ],
    Mobile: [
        'Smartphone', 'Phone Charger', 'Wireless Charger', 'Car Mount',
        'Selfie Stick', 'Phone Stand', 'PopSocket', 'Battery Pack'
    ],
    Wearables: [
        'Smart Watch', 'Fitness Tracker', 'Smart Ring', 'Smart Glasses',
        'Heart Rate Monitor', 'GPS Watch', 'Sleep Tracker'
    ],
    'Smart Home': [
        'Smart Bulb', 'Smart Plug', 'Security Camera', 'Video Doorbell',
        'Smart Lock', 'Smart Thermostat', 'Motion Sensor', 'Smart Switch'
    ],
    Camera: [
        'DSLR Camera', 'Mirrorless Camera', 'Action Camera', 'Camera Lens',
        'Tripod', 'Camera Bag', 'SD Card', 'Flash', 'Gimbal'
    ],
    Networking: [
        'Router', 'WiFi Extender', 'Network Switch', 'Modem',
        'Ethernet Cable', 'Powerline Adapter', 'Access Point'
    ],
    Storage: [
        'External SSD', 'External HDD', 'NAS Drive', 'USB Flash Drive',
        'SD Card Reader', 'Hard Drive Enclosure', 'Cloud Storage Device'
    ]
};

// Adjectives for product variations
const adjectives = [
    'Pro', 'Plus', 'Max', 'Ultra', 'Premium', 'Elite', 'Advanced',
    'Deluxe', 'Professional', 'HD', '4K', 'Wireless', 'Portable',
    'Compact', 'Mini', 'XL', 'RGB', 'Ergonomic', 'Mechanical'
];

// Brands
const brands = [
    'TechPro', 'InnovateTech', 'SmartGear', 'ProElectronics', 'EliteTech',
    'DigitalWave', 'FutureTech', 'PrimeTech', 'CoreTech', 'NexGen',
    'PowerMax', 'UltraGear', 'SwiftTech', 'PrecisionTech', 'VisionTech',
    'SonicWave', 'FlexTech', 'SpeedTech', 'ImpactTech', 'ApexTech'
];

/**
 * Generate a random integer between min and max (inclusive)
 */
function randomInt(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
}

/**
 * Pick a random element from an array
 */
function randomChoice(array) {
    return array[Math.floor(Math.random() * array.length)];
}

/**
 * Generate a realistic price
 */
function generatePrice() {
    const prices = [
        9.99, 12.99, 14.99, 19.99, 24.99, 29.99, 34.99, 39.99,
        49.99, 59.99, 69.99, 79.99, 89.99, 99.99, 119.99, 149.99,
        199.99, 249.99, 299.99, 349.99, 399.99, 449.99, 499.99,
        599.99, 699.99, 799.99, 899.99, 999.99, 1299.99, 1499.99,
        1999.99, 2499.99
    ];
    return randomChoice(prices);
}

/**
 * Generate a product description
 */
function generateDescription(productName, category) {
    const templates = [
        `High-quality ${productName.toLowerCase()} designed for ${category.toLowerCase()} enthusiasts.`,
        `Premium ${productName.toLowerCase()} with advanced features and superior performance.`,
        `Professional-grade ${productName.toLowerCase()} perfect for work and play.`,
        `Innovative ${productName.toLowerCase()} combining style and functionality.`,
        `Top-rated ${productName.toLowerCase()} with excellent build quality and durability.`,
        `Feature-rich ${productName.toLowerCase()} offering exceptional value for money.`,
        `Cutting-edge ${productName.toLowerCase()} with the latest technology.`,
        `Reliable ${productName.toLowerCase()} backed by industry-leading warranty.`,
        `Versatile ${productName.toLowerCase()} suitable for both professionals and beginners.`,
        `Award-winning ${productName.toLowerCase()} with outstanding reviews.`
    ];
    return randomChoice(templates);
}

/**
 * Generate a single product
 */
function generateProduct(index) {
    const category = randomChoice(categories);
    const productType = randomChoice(productTypes[category] || productTypes.Electronics);
    const adjective = Math.random() > 0.4 ? randomChoice(adjectives) + ' ' : '';
    const variant = randomInt(1000, 9999);

    const name = `${adjective}${productType} ${variant}`;
    const brand = randomChoice(brands);
    const description = generateDescription(productType, category);
    const price = generatePrice();
    const stock = randomInt(0, 500);

    return {
        name,
        description,
        price,
        category,
        stock_quantity: stock
    };
}

/**
 * Escape single quotes for SQL
 */
function escapeSql(str) {
    if (!str) return str;
    return str.replace(/'/g, "''");
}

/**
 * Generate SQL INSERT statements
 */
function generateSQL(products) {
    const lines = [
        '-- Generated products data',
        '-- Total products: ' + products.length,
        '-- Generated at: ' + new Date().toISOString(),
        '',
        'SET search_path TO products, public;',
        '',
        '-- Insert products in batches for better performance',
        'INSERT INTO products (name, description, price, category, stock_quantity) VALUES'
    ];

    const valueLines = products.map((product, index) => {
        const isLast = index === products.length - 1;
        const name = escapeSql(product.name);
        const desc = escapeSql(product.description);
        const category = escapeSql(product.category);

        return `    ('${name}', '${desc}', ${product.price}, '${category}', ${product.stock_quantity})${isLast ? ';' : ','}`;
    });

    return lines.concat(valueLines).join('\n');
}

/**
 * Main function
 */
function main() {
    console.log('Generating 2000 products...');

    const products = [];
    for (let i = 0; i < 2000; i++) {
        products.push(generateProduct(i));
        if ((i + 1) % 500 === 0) {
            console.log(`Generated ${i + 1} products...`);
        }
    }

    console.log('Generating SQL...');
    const sql = generateSQL(products);

    const outputPath = path.join(__dirname, 'data', 'products_data.sql');
    fs.writeFileSync(outputPath, sql, 'utf8');

    console.log(`✓ Successfully generated ${products.length} products`);
    console.log(`✓ SQL file saved to: ${outputPath}`);
    console.log(`✓ File size: ${(sql.length / 1024).toFixed(2)} KB`);
}

// Run the script
main();
