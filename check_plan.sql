-- Check if the premium plan with the given product ID exists
SELECT * FROM dbquizapp.premium_plans WHERE mayar_product_id = 'aedda011-50a6-4839-9109-3e413ad5b887';

-- Check if the plan with ID 5 exists
SELECT * FROM dbquizapp.premium_plans WHERE id = 5;

-- Check the structure of the user_subscriptions table
DESCRIBE dbquizapp.user_subscriptions; 