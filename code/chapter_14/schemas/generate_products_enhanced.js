#!/usr/bin/env node

/**
 * Generate 2000 enhanced product records with categories and specifications
 * Outputs SQL INSERT statements to schemas/data/products_data_enhanced.sql
 */

const fs = require('fs');
const path = require('path');

// Categories with their IDs (matching the migration seed data)
const categories = [
    { id: 1, name: 'Electronics', slug: 'electronics' },
    { id: 2, name: 'Accessories', slug: 'accessories' },
    { id: 3, name: 'Gaming', slug: 'gaming' },
    { id: 4, name: 'Office', slug: 'office' },
    { id: 5, name: 'Audio', slug: 'audio' },
    { id: 6, name: 'Computers', slug: 'computers' },
    { id: 7, name: 'Mobile', slug: 'mobile' },
    { id: 8, name: 'Wearables', slug: 'wearables' },
    { id: 9, name: 'Smart Home', slug: 'smart-home' },
    { id: 10, name: 'Camera', slug: 'camera' },
    { id: 11, name: 'Networking', slug: 'networking' },
    { id: 12, name: 'Storage', slug: 'storage' }
];

// Product types per category
const productTypes = {
    'Electronics': ['Monitor', 'Laptop', 'Desktop', 'Tablet', 'E-Reader', 'TV', 'Projector'],
    'Accessories': ['USB-C Cable', 'HDMI Cable', 'Power Adapter', 'Phone Case', 'Screen Protector'],
    'Gaming': ['Gaming Mouse', 'Gaming Keyboard', 'Gaming Headset', 'Controller', 'Gaming Chair', 'Mouse Pad'],
    'Office': ['Desk Lamp', 'Office Chair', 'Standing Desk', 'Whiteboard', 'Desk Organizer'],
    'Audio': ['Bluetooth Speaker', 'Headphones', 'Earbuds', 'Microphone', 'Sound Bar'],
    'Computers': ['Keyboard', 'Mouse', 'Webcam', 'External Drive', 'RAM Module', 'SSD'],
    'Mobile': ['Smartphone', 'Phone Charger', 'Wireless Charger', 'Car Mount', 'Battery Pack'],
    'Wearables': ['Smart Watch', 'Fitness Tracker', 'Smart Ring', 'Smart Glasses'],
    'Smart Home': ['Smart Bulb', 'Smart Plug', 'Security Camera', 'Video Doorbell', 'Smart Lock'],
    'Camera': ['DSLR Camera', 'Mirrorless Camera', 'Action Camera', 'Camera Lens', 'Tripod'],
    'Networking': ['Router', 'WiFi Extender', 'Network Switch', 'Modem', 'Ethernet Cable'],
    'Storage': ['External SSD', 'External HDD', 'NAS Drive', 'USB Flash Drive', 'SD Card Reader']
};

const adjectives = ['Pro', 'Plus', 'Max', 'Ultra', 'Premium', 'Elite', 'Advanced', 'Deluxe',
                    'HD', '4K', 'Wireless', 'Portable', 'Compact', 'Mini', 'XL', 'RGB'];

const brands = ['TechPro', 'InnovateTech', 'SmartGear', 'ProElectronics', 'EliteTech',
                'DigitalWave', 'FutureTech', 'PrimeTech', 'CoreTech', 'NexGen'];

const colors = [['Black'], ['White'], ['Silver'], ['Black', 'White'], ['Blue', 'Red'],
                ['Gray', 'Black', 'White'], null];

const sizes = [['S', 'M', 'L'], ['XS', 'S', 'M', 'L', 'XL'], ['One Size'], null];

// Specifications templates per category
const specTemplates = {
    'Electronics': [
        ['Display Size', ['24"', '27"', '32"', '43"']],
        ['Resolution', ['1080p', '4K', '2K', '8K']],
        ['Refresh Rate', ['60Hz', '120Hz', '144Hz']],
        ['Warranty', ['1 Year', '2 Years', '3 Years']]
    ],
    'Gaming': [
        ['DPI', ['800-3200', '1000-4000', '400-6400']],
        ['Connection', ['Wireless', 'Wired', 'Both']],
        ['RGB Lighting', ['Yes', 'No']],
        ['Warranty', ['1 Year', '2 Years']]
    ],
    'Audio': [
        ['Driver Size', ['40mm', '50mm', '60mm']],
        ['Frequency Response', ['20Hz-20kHz', '10Hz-40kHz']],
        ['Impedance', ['32 Ohms', '80 Ohms', '250 Ohms']],
        ['Warranty', ['1 Year', '2 Years']]
    ],
    'Computers': [
        ['Interface', ['USB 3.0', 'USB-C', 'Thunderbolt']],
        ['Capacity', ['256GB', '512GB', '1TB', '2TB']],
        ['Speed', ['3000MB/s', '5000MB/s', '7000MB/s']],
        ['Warranty', ['3 Years', '5 Years']]
    ]
};

function randomInt(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
}

function randomChoice(array) {
    return array[Math.floor(Math.random() * array.length)];
}

function generateASIN() {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
    return Array.from({length: 10}, () => chars[randomInt(0, chars.length - 1)]).join('');
}

function generateSKU(category, index) {
    return `${category.slug.toUpperCase().replace('-', '')}-${index.toString().padStart(6, '0')}`;
}

function generatePrice() {
    const prices = [9.99, 12.99, 14.99, 19.99, 24.99, 29.99, 34.99, 39.99, 49.99, 59.99,
                    69.99, 79.99, 89.99, 99.99, 119.99, 149.99, 199.99, 249.99, 299.99,
                    349.99, 399.99, 449.99, 499.99, 599.99, 699.99, 799.99, 899.99,
                    999.99, 1299.99, 1499.99, 1999.99, 2499.99];
    return randomChoice(prices);
}

function generateDescription(productName, category) {
    const templates = [
        `High-quality ${productName.toLowerCase()} designed for ${category.name.toLowerCase()} enthusiasts.`,
        `Premium ${productName.toLowerCase()} with advanced features and superior performance.`,
        `Professional-grade ${productName.toLowerCase()} perfect for work and play.`,
        `Innovative ${productName.toLowerCase()} combining style and functionality.`,
        `Top-rated ${productName.toLowerCase()} with excellent build quality.`
    ];
    return randomChoice(templates);
}

function generateProduct(index) {
    const category = randomChoice(categories);
    const productType = randomChoice(productTypes[category.name]);
    const adjective = Math.random() > 0.5 ? randomChoice(adjectives) + ' ' : '';
    const variant = randomInt(1000, 9999);

    const productName = `${adjective}${productType} ${variant}`;
    const brand = randomChoice(brands);
    const description = generateDescription(productType, category);
    const price = generatePrice();
    const stock = randomInt(0, 500);
    const asin = generateASIN();
    const sku = generateSKU(category, index);

    return {
        asin,
        sku,
        product_name: productName,
        brand,
        description,
        category_id: category.id,
        category_name: category.name,
        price,
        stock_quantity: stock,
        sizes: randomChoice(sizes),
        colors: randomChoice(colors),
        available_for_delivery: Math.random() > 0.1,
        available_for_pickup: Math.random() > 0.7,
        free_returns: Math.random() > 0.5
    };
}

function generateSpecs(product, productIndex) {
    const category = categories.find(c => c.id === product.category_id);
    const templates = specTemplates[category.name] || [];

    if (templates.length === 0) return [];

    const specs = [];
    const numSpecs = randomInt(2, Math.min(4, templates.length));

    const selectedTemplates = [];
    while (selectedTemplates.length < numSpecs && selectedTemplates.length < templates.length) {
        const template = randomChoice(templates);
        if (!selectedTemplates.includes(template)) {
            selectedTemplates.push(template);
        }
    }

    selectedTemplates.forEach(([name, values]) => {
        specs.push({
            product_index: productIndex + 1,
            spec_name: name,
            spec_value: randomChoice(values)
        });
    });

    return specs;
}

function escapeSql(str) {
    if (!str) return str;
    return str.replace(/'/g, "''");
}

function arrayToPostgres(arr) {
    if (!arr) return 'NULL';
    return `ARRAY[${arr.map(item => `'${escapeSql(item)}'`).join(', ')}]`;
}

function generateSQL(products, specifications) {
    const lines = [
        '-- Generated enhanced products data',
        '-- Total products: ' + products.length,
        '-- Total specifications: ' + specifications.length,
        '-- Generated at: ' + new Date().toISOString(),
        '',
        'SET search_path TO products, public;',
        '',
        '-- Insert products',
        'INSERT INTO products (asin, sku, product_name, brand, description, category_id, price, stock_quantity, sizes, colors, available_for_delivery, available_for_pickup, free_returns) VALUES'
    ];

    const productLines = products.map((product, index) => {
        const isLast = index === products.length - 1;
        const name = escapeSql(product.product_name);
        const desc = escapeSql(product.description);
        const brand = escapeSql(product.brand);
        const asin = escapeSql(product.asin);
        const sku = escapeSql(product.sku);

        return `    ('${asin}', '${sku}', '${name}', '${brand}', '${desc}', ${product.category_id}, ${product.price}, ${product.stock_quantity}, ${arrayToPostgres(product.sizes)}, ${arrayToPostgres(product.colors)}, ${product.available_for_delivery}, ${product.available_for_pickup}, ${product.free_returns})${isLast ? ';' : ','}`;
    });

    lines.push(...productLines);
    lines.push('');
    lines.push('-- Insert product specifications');
    lines.push('INSERT INTO product_specifications (product_id, spec_name, spec_value) VALUES');

    const specLines = specifications.map((spec, index) => {
        const isLast = index === specifications.length - 1;
        const name = escapeSql(spec.spec_name);
        const value = escapeSql(spec.spec_value);
        return `    (${spec.product_index}, '${name}', '${value}')${isLast ? ';' : ','}`;
    });

    lines.push(...specLines);

    return lines.join('\n');
}

function main() {
    console.log('Generating 2000 enhanced products...');

    const products = [];
    const specifications = [];

    for (let i = 0; i < 2000; i++) {
        const product = generateProduct(i);
        products.push(product);

        // Generate 2-4 specifications for each product
        const specs = generateSpecs(product, i);
        specifications.push(...specs);

        if ((i + 1) % 500 === 0) {
            console.log(`Generated ${i + 1} products with ${specifications.length} specs...`);
        }
    }

    console.log('Generating SQL...');
    const sql = generateSQL(products, specifications);

    const outputPath = path.join(__dirname, 'data', 'products_data_enhanced.sql');
    fs.writeFileSync(outputPath, sql, 'utf8');

    console.log(`✓ Successfully generated ${products.length} products`);
    console.log(`✓ Successfully generated ${specifications.length} specifications`);
    console.log(`✓ SQL file saved to: ${outputPath}`);
    console.log(`✓ File size: ${(sql.length / 1024).toFixed(2)} KB`);

    // Generate category distribution
    const distribution = {};
    products.forEach(p => {
        const cat = categories.find(c => c.id === p.category_id).name;
        distribution[cat] = (distribution[cat] || 0) + 1;
    });
    console.log('\nCategory distribution:');
    Object.entries(distribution).sort((a, b) => b[1] - a[1]).forEach(([cat, count]) => {
        console.log(`  ${cat}: ${count} products`);
    });
}

main();
