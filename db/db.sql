CREATE DATABASE rms;

CREATE TABLE Rooms (
    id INT PRIMARY KEY NOT NULL,
    name VARCHAR(255) NOT NULL,
    rent DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE Tenants (
    id INT PRIMARY KEY NOT NULL,
    room_id INT NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    FOREIGN KEY (room_id) REFERENCES Rooms(id)
);

CREATE TABLE Electricity_Readings (
    id INT PRIMARY KEY NOT NULL,
    tenant_id INT NOT NULL,
    prev_reading INT NOT NULL,
    curr_reading INT NOT NULL,
    consumption DECIMAL(10,2) GENERATED ALWAYS AS (curr_reading - prev_reading) STORED NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES Tenants(id)
);


CREATE TABLE Bills (
    id INT PRIMARY KEY NOT NULL,
    tenant_id INT NOT NULL,
    room_charges DECIMAL(10,2) DEFAULT 0 NOT NULL,
    electric_charges DECIMAL(10,2) DEFAULT 0 NOT NULL,
    additional_charges DECIMAL(10,2) DEFAULT 0 NOT NULL,
    additional_description VARCHAR(255),
    total_amount DECIMAL(10,2) GENERATED ALWAYS AS (room_charges + electric_charges + additional_charges) STORED NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES Tenants(id)
);

-- Insert 5 rooms into the Rooms table
INSERT INTO Rooms (id, name, rent) VALUES 
(1, 'Room A', 500.00),
(2, 'Room B', 600.00),
(3, 'Room C', 550.00),
(4, 'Room D', 700.00),
(5, 'Room E', 650.00);

-- Insert 5 tenants into the Tenants table
INSERT INTO Tenants (id, room_id, name) VALUES 
(1, 1, 'John Doe'),
(2, 2, 'Jane Smith'),
(3, 3, 'Michael Johnson'),
(4, 4, 'Emily Davis'),
(5, 5, 'Daniel Brown');

-- Insert 5 electricity readings into the Electricity_Readings table
INSERT INTO Electricity_Readings (id, tenant_id, prev_reading, curr_reading) VALUES 
(1, 1, 100, 150),
(2, 2, 200, 250),
(3, 3, 50, 100),
(4, 4, 300, 350),
(5, 5, 400, 450);

-- Insert 5 bills into the Bills table
INSERT INTO Bills (id, tenant_id, room_charges, electric_charges, additional_charges, additional_description) VALUES 
(1, 1, 500.00, 50.00, 10.00, 'Late payment fee'),
(2, 2, 600.00, 60.00, 20.00, 'Cleaning service'),
(3, 3, 550.00, 75.00, 0.00, 'No additional charges'),
(4, 4, 700.00, 80.00, 15.00, 'Maintenance fee'),
(5, 5, 650.00, 90.00, 5.00, 'Extra usage');
